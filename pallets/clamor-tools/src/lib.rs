#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

// #[cfg(test)]
// mod tests;

pub use pallet::*;
use codec::{Decode, Encode};
use sp_std::{collections::btree_set::BTreeSet, vec, vec::Vec};
use sp_chainblocks::{Hash256, SupportedChains, KEY_TYPE};
use sp_runtime::MultiSigner;
use sp_core::{ed25519, ecdsa, U256};
use sp_io::{
	crypto as Crypto,
	hashing::keccak_256,
	offchain_index,
};
use sp_runtime::offchain::storage::StorageValueRef;
use frame_system::offchain::{
	CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
	SigningTypes,
};
use frame_system::offchain::AppCrypto;

#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct DetachRequest {
	pub fragment_hash: Hash256,
	pub target_chain: SupportedChains,
	pub target_account: Vec<u8>, // an eth address or so
}
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct DetachInternalData<TPublic> {
	pub public: TPublic,
	pub fragment_hash: Hash256,
	pub target_chain: SupportedChains,
	pub target_account: Vec<u8>, // an eth address or so
	pub remote_signature: Vec<u8>,
	pub nonce: u64,
}

#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct ExportData {
	pub chain: SupportedChains,
	pub owner: Vec<u8>,
	pub nonce: u64,
}

impl<T: SigningTypes> SignedPayload<T> for DetachInternalData<T::Public> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use sp_chainblocks::FragmentOwner;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
	CreateSignedTransaction<Call<Self>>
	+ pallet_randomness_collective_flip::Config
	+ frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub eth_authorities: Vec<ecdsa::Public>,
		pub keys: Vec<ed25519::Public>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { eth_authorities: Vec::new(), keys: Vec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			Pallet::<T>::initialize_eth_authorities(&self.eth_authorities);
			Pallet::<T>::initialize_keys(&self.keys);
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// fragment-hash to entity-hash-sequence
	#[pallet::storage]
	pub type DetachRequests<T: Config> = StorageValue<_, Vec<DetachRequest>, ValueQuery>;

	#[pallet::storage]
	pub type DetachedFragments<T: Config> = StorageMap<_, Identity, Hash256, ExportData>;

	#[pallet::storage]
	pub type DetachNonces<T: Config> = StorageDoubleMap<_, Blake2_128Concat, Vec<u8>, Blake2_128Concat, SupportedChains, u64>;

	#[pallet::storage]
	pub type EthereumAuthorities<T: Config> = StorageValue<_, BTreeSet<ecdsa::Public>, ValueQuery>;

	#[pallet::storage]
	pub type FragKeys<T: Config> = StorageValue<_, BTreeSet<ed25519::Public>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Detached(Hash256, Vec<u8>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Failed to sign message
		SigningFailed,
		/// No Validators are present
		NoValidator,
		/// Not the owner of the fragment
		Unauthorized,
		/// Fragment is already detached
		FragmentDetached,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(25_000)]
		pub fn add_eth_auth(origin: OriginFor<T>, public: ecdsa::Public) -> DispatchResult {
			ensure_root(origin)?;

			log::debug!("New eth auth: {:?}", public);

			<EthereumAuthorities<T>>::mutate(|validators| {
				validators.insert(public);
			});

			Ok(())
		}

		// Remove validator public key to the list
		#[pallet::weight(25_000)]
		pub fn del_eth_auth(origin: OriginFor<T>, public: ecdsa::Public) -> DispatchResult {
			ensure_root(origin)?;

			log::debug!("Removed eth auth: {:?}", public);

			<EthereumAuthorities<T>>::mutate(|validators| {
				validators.remove(&public);
			});

			Ok(())
		}

		#[pallet::weight(25_000)]
		pub fn add_key(origin: OriginFor<T>, public: ed25519::Public) -> DispatchResult {
			ensure_root(origin)?;

			log::debug!("New key: {:?}", public);

			<FragKeys<T>>::mutate(|validators| {
				validators.insert(public);
			});

			Ok(())
		}

		#[pallet::weight(25_000)]
		pub fn del_key(origin: OriginFor<T>, public: ed25519::Public) -> DispatchResult {
			ensure_root(origin)?;

			log::debug!("Removed key: {:?}", public);

			<FragKeys<T>>::mutate(|validators| {
				validators.remove(&public);
			});

			Ok(())
		}

		/// Detached a fragment from this chain by emitting an event that includes a signature.
		/// The remote target chain can attach this fragment by using this signature.
		#[pallet::weight(25_000)]
		pub fn internal_finalize_detach(
			origin: OriginFor<T>,
			data: DetachInternalData<T::Public>,
			_signature: T::Signature,
		) -> DispatchResult {

			ensure_none(origin)?;

			// Update nonce
			<DetachNonces<T>>::insert(&data.target_account, data.target_chain.clone(), data.nonce);

			let export_data = ExportData {
				chain: data.target_chain,
				owner: data.target_account,
				nonce: data.nonce,
			};

			// add to Detached fragments map
			<DetachedFragments<T>>::insert(data.fragment_hash, export_data);

			// emit event
			Self::deposit_event(Event::Detached(
				data.fragment_hash,
				data.remote_signature.clone(),
			));

			log::debug!(
				"Detached fragment with hash: {:?} signature: {:?}",
				data.fragment_hash,
				data.remote_signature
			);

			Ok(())
		}


		/// Detached a fragment from this chain by emitting an event that includes a signature.
		/// The remote target chain can attach this fragment by using this signature.
		#[pallet::weight(25_000)]
		pub fn detach(
			origin: OriginFor<T>,
			fragment_hash: Hash256,
			target_chain: SupportedChains,
			target_account: Vec<u8>, // an eth address or so
			owner: FragmentOwner<T::AccountId>
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match owner {
				FragmentOwner::User(owner) => ensure!(owner == who, Error::<T>::Unauthorized),
				FragmentOwner::ExternalAsset(_ext_asset) =>
				// We don't allow detaching external assets
					ensure!(false, Error::<T>::Unauthorized),
			};

			ensure!(
				!<DetachedFragments<T>>::contains_key(&fragment_hash),
				Error::<T>::FragmentDetached
			);

			<DetachRequests<T>>::mutate(|requests| {
				requests.push(DetachRequest { fragment_hash, target_chain, target_account });
			});

			Ok(())
		}

	}


	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: T::BlockNumber) {
			// drain and process requests
			let requests = <DetachRequests<T>>::take();
			if !requests.is_empty() {
				log::debug!("Got {} detach requests", requests.len());
				offchain_index::set(b"fragments-detach-requests", &requests.encode());
			}
		}

		fn offchain_worker(_n: T::BlockNumber) {
			<Pallet<T>>::process_detach_requests();
		}
	}


	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			if let Call::internal_finalize_detach { ref data, ref signature } = call {
				// check public is valid
				let valid_keys = <FragKeys<T>>::get();
				log::debug!("Valid keys: {:?}", valid_keys);
				// I'm sure there is a way to do this without serialization but I can't spend so
				// much time fighting with rust
				let pub_key = data.public.encode();
				let pub_key: ed25519::Public = {
					if let Ok(MultiSigner::Ed25519(pub_key)) =
					<MultiSigner>::decode(&mut &pub_key[..])
					{
						pub_key
					} else {
						return InvalidTransaction::BadSigner.into()
					}
				};
				log::debug!("Public key: {:?}", pub_key);
				if !valid_keys.contains(&pub_key) {
					return InvalidTransaction::BadSigner.into()
				}
				// most expensive bit last
				let signature_valid =
					SignedPayload::<T>::verify::<T::AuthorityId>(data, signature.clone());
				if !signature_valid {
					return InvalidTransaction::BadProof.into()
				}
				log::debug!("Sending detach finalization extrinsic");
				ValidTransaction::with_tag_prefix("Fragments-Detach")
					.and_provides(data.fragment_hash)
					.and_provides(data.target_chain.clone())
					.and_provides(data.target_account.clone())
					.and_provides(data.nonce)
					.longevity(5)
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn initialize_eth_authorities(authorities: &[ecdsa::Public]) {
			if !authorities.is_empty() {
				assert!(
					<EthereumAuthorities<T>>::get().is_empty(),
					"EthereumAuthorities are already initialized!"
				);
				for authority in authorities {
					<EthereumAuthorities<T>>::mutate(|authorities| {
						authorities.insert(authority.clone());
					});
				}
			}
		}

		fn initialize_keys(keys: &[ed25519::Public]) {
			if !keys.is_empty() {
				assert!(<FragKeys<T>>::get().is_empty(), "FragKeys are already initialized!");
				for key in keys {
					<FragKeys<T>>::mutate(|keys| {
						keys.insert(*key);
					});
				}
			}
		}

		fn process_detach_requests() {
			const FAILED: () = ();
			let requests = StorageValueRef::persistent(b"fragments-detach-requests");
			let _ = requests.mutate(|requests: Result<Option<Vec<DetachRequest>>, _>| match requests {
				Ok(Some(requests)) => {
					log::debug!("Got {} detach requests", requests.len());
					for request in requests {
						let chain_id = match request.target_chain {
							SupportedChains::EthereumMainnet => U256::from(1),
							SupportedChains::EthereumRinkeby => U256::from(4),
							SupportedChains::EthereumGoerli => U256::from(5),
						};

						let values = match request.target_chain {
							SupportedChains::EthereumMainnet |
							SupportedChains::EthereumRinkeby |
							SupportedChains::EthereumGoerli => {
								// check if we need to generate new ecdsa keys
								let ed_keys = Crypto::ed25519_public_keys(KEY_TYPE);
								let keys_ref = StorageValueRef::persistent(b"fragments-frag-ecdsa-keys");
								let keys = keys_ref.get::<BTreeSet<ed25519::Public>>().unwrap_or_default();
								let mut keys = if let Some(keys) = keys {
									keys
								} else {
									BTreeSet::new()
								};
								// doing this cos mutate was insane...
								let mut edited = false;
								for ed_key in &ed_keys {
									if !keys.contains(ed_key) {
										let signed = Crypto::ed25519_sign(KEY_TYPE, ed_key, b"fragments-frag-ecdsa-keys").unwrap();
										let key = keccak_256(&signed.0[..]);
										let mut key_hex = [0u8; 64];
										hex::encode_to_slice(key, &mut key_hex).map_err(|_| FAILED)?;
										let key_hex = [b"0x", &key_hex[..]].concat();
										log::debug!("Adding new key from seed: {:?}", key_hex);
										let _public = Crypto::ecdsa_generate(KEY_TYPE, Some(key_hex));
										keys.insert(*ed_key);
										edited = true;
									}
								}
								if edited {
									// commit it back
									keys_ref.set(&keys);
								}
								// get local keys
								let keys = Crypto::ecdsa_public_keys(KEY_TYPE);
								log::debug!("ecdsa local keys {:x?}", keys);
								// make sure the local key is in the global authorities set!
								let key = keys
									.iter()
									.find(|k| <EthereumAuthorities<T>>::get().contains(k));
								if let Some(key) = key {
									// This is critical, we send over to the ethereum smart
									// contract this signature The ethereum smart contract call
									// will be the following attach(fragment_hash, local_owner,
									// signature, clamor_nonce); on this target chain the nonce
									// needs to be exactly the same as the one here
									let mut payload = request.fragment_hash.encode();
									let mut chain_id_be: [u8; 32] = [0u8; 32];
									chain_id.to_big_endian(&mut chain_id_be);
									payload.extend(&chain_id_be[..]);
									let mut target_account: [u8; 20] = [0u8; 20];
									if request.target_account.len() != 20 {
										return Err(FAILED);
									}
									target_account.copy_from_slice(&request.target_account[..]);
									payload.extend(&target_account[..]);
									let nonce = <DetachNonces<T>>::get(
										&request.target_account,
										request.target_chain.clone(),
									);
									let nonce = if let Some(nonce) = nonce {
										// add 1, remote will add 1
										let nonce = nonce.checked_add(1).unwrap();
										payload.extend(nonce.to_be_bytes());
										nonce // for storage
									} else {
										// there never was a nonce
										payload.extend(1u64.to_be_bytes());
										1u64
									};
									log::debug!("payload: {:x?}, len: {}", payload, payload.len());
									let payload = keccak_256(&payload);
									log::debug!("payload hash: {:x?}, len: {}", payload, payload.len());
									let msg = [
										b"\x19Ethereum Signed Message:\n32",
										&payload[..],
									]
										.concat();
									let msg = keccak_256(&msg);
									// Sign the payload with a trusted validation key
									let signature = Crypto::ecdsa_sign_prehashed(KEY_TYPE, key, &msg);
									if let Some(signature) = signature {
										// No more failures from this path!!
										let mut signature = signature.0.to_vec();
										// fix signature ending for ethereum
										signature[64] += 27u8;
										Ok((signature, nonce))
									} else {
										Err(Error::<T>::SigningFailed)
									}
								} else {
									Err(Error::<T>::NoValidator)
								}
							},
						};


						match values {
							Ok((signature, nonce)) => {
								// exec unsigned transaction from here
								log::debug!("Executing unsigned transaction for detach; signature: {:x?}, nonce: {}", signature, nonce);
								if let Err(e) = Signer::<T, T::AuthorityId>::any_account()
									.send_unsigned_transaction(
										|account| DetachInternalData {
											public: account.public.clone(),
											fragment_hash: request.fragment_hash,
											target_chain: request.target_chain.clone(),
											target_account: request.target_account.clone(),
											remote_signature: signature.clone(),
											nonce,
										},
										|payload, signature| Call::internal_finalize_detach {
											data: payload,
											signature,
										},
									)
									.ok_or("No local accounts accounts available.") {
									log::error!("Failed to send unsigned detach transaction with error: {:?}", e);
								}
							},
							Err(e) => {log::debug!("Failed to detach with error {:?}", e)},
						}
					}
					Ok(vec![])
				},
				_ => Err(FAILED),
			});
		}
	}
}
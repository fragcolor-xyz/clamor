//! This pallet `fragments` performs logic related to Proto-Fragments.
//! 
//! IMPORTANT STUFF TO KNOW:
//! 
//! A Proto-Fragment is a digital asset that can be used to build a game or application

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod dummy_data;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod tests;

mod weights;

use protos::categories::Categories;

use sp_core::{ecdsa, H160, U256};

use codec::{Compact, Decode, Encode};
pub use pallet::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_io::{hashing::blake2_256, transaction_index};
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};

pub use weights::WeightInfo;

use sp_clamor::{get_locked_frag_account, Hash256};

use scale_info::prelude::{
	format,
	string::{String, ToString},
};
use serde_json::{json, Map, Value};

use base58::ToBase58;

/// ¿
#[derive(Encode, Decode, Clone, PartialEq, Debug, Eq, scale_info::TypeInfo)]
pub enum LinkSource {
	// Generally we just store this data, we don't verify it as we assume auth service did it.
	// (Link signature, Linked block number, EIP155 Chain ID)
	Evm(ecdsa::Signature, u64, U256),
}

/// **Types** of **Assets that are linked to a Proto-Fragment** (e.g an ERC-721 Contract etc.)
#[derive(Encode, Decode, Clone, PartialEq, Debug, Eq, scale_info::TypeInfo)]
pub enum LinkedAsset {
	// Ethereum (ERC721 Contract address, Token ID, Link source)
	Erc721(H160, U256, LinkSource),
}

/// **Types** of **Proto-Fragment Owners**
#[derive(Encode, Decode, Clone, PartialEq, Debug, Eq, scale_info::TypeInfo)]
pub enum ProtoOwner<TAccountId> {
	// A **regular account** on **this chain**
	User(TAccountId),
	// An **external asset** not on this chain
	ExternalAsset(LinkedAsset),
}

/// **Data Type** used to **Query and Filter for Proto-Fragments**
#[derive(Encode, Decode, Clone, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct GetProtosParams<TAccountId, TString> {
	pub desc: bool,
	pub from: u64,
	pub limit: u64,
	pub metadata_keys: Vec<TString>,
	pub owner: Option<TAccountId>,
	pub return_owners: bool,
	pub categories: Vec<Categories>,
	pub tags: Vec<TString>,
	pub available: Option<bool>,
}

/// **Struct** of a **Proto-Fragment Patch**
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub struct ProtoPatch<TBlockNumber> {
	/// **Block Number** in which the **patch was created**
	pub block: TBlockNumber,
	/// **Hash** of patch data
	pub data_hash: Hash256,
	/// **List of New Proto-Fragments** that was **used** to **create** the **patch** (INCDT)
	pub references: Vec<Hash256>,
}

#[derive(Default, Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub struct AccountsInfo {
	pub active_accounts: u128,
	pub lifetime_accounts: u128,
}

/// **Struct** of a **Proto-Fragment**
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq, Eq)]
pub struct Proto<TAccountId, TBlockNumber> {
	/// **Block Number** in which the **Proto-Fragment was minted in**
	pub block: TBlockNumber,
	/// **List of *ProtoPatch* structs** of the **Proto-Fragment**
	pub patches: Vec<ProtoPatch<TBlockNumber>>,
	/// **Amount of FRAG tokens** that **need to be staked on the Proto-Fragment** to **use the Proto-Fragment in other Proto-Fragments** (*optional*)
	/// NOTE: If None, the **Proto-Fragment** *<u>can't be used</u>* in **other Proto-Fragments**
	pub include_cost: Option<Compact<u64>>,
	/// **Original Creator** of the **Proto-Fragment**
	pub creator: TAccountId,
	/// *Current Owner** of the **Proto-Fragment**
	pub owner: ProtoOwner<TAccountId>,
	/// **List of other Proto-Fragments** used to create the **Proto-Fragment**
	pub references: Vec<Hash256>,
	/// **Category** of the **Proto-Fragment**
	pub category: Categories,
	/// **List of Tags** associated with the **Proto-Fragment**
	pub tags: Vec<Compact<u64>>,
	/// **Map** that maps the **Key of a Proto-Fragment's Metadata Object** to the **Hash of the aforementioned Metadata Object**
	pub metadata: BTreeMap<Compact<u64>, Hash256>,
	/// Accounts information for this proto.
	pub accounts_info: AccountsInfo,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, Twox64Concat};
	use frame_system::pallet_prelude::*;
	use pallet_detach::{DetachRequest, DetachRequests, DetachedHashes, SupportedChains};
	use sp_clamor::CID_PREFIX;
	use sp_runtime::{traits::Saturating, SaturatedConversion};

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_detach::Config + pallet_accounts::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type StorageBytesMultiplier: Get<u64>;

		/// The **Lock-Up Period" for **Staking FRAG Tokens**
		#[pallet::constant]
		type StakeLockupPeriod: Get<u64>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// **StorageMap** that maps a **Tag (of type `Vec<u8>`)** to an **index number**
	#[pallet::storage]
	pub type Tags<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, u64>;

	/// **StorageValue** that **equals** the **total number of unique tags in the blockchain**
	#[pallet::storage]
	pub type TagsIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// **StorageMap** that maps a **Metadata Key (of type `Vec<u8>`)** to an **index number**
	#[pallet::storage]
	pub type MetaKeys<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, u64>;

	/// **StorageValue** that **equals** the **total number of unique Metadata Keys in the blockchain**
	#[pallet::storage]
	pub type MetaKeysIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// **StorageMap** that maps a **Proto-Fragment's data's hash** to a ***Proto* struct (of the aforementioned Proto-Fragment)**
	#[pallet::storage]
	pub type Protos<T: Config> =
		StorageMap<_, Identity, Hash256, Proto<T::AccountId, T::BlockNumber>>;

	/// **StorageMap** that maps a **variant of the *Category* enum** to a **list of Proto-Fragment hashes (that have the aforementioned variant)**
	// Not ideal but to have it iterable...
	#[pallet::storage]
	pub type ProtosByCategory<T: Config> = StorageMap<_, Twox64Concat, Categories, Vec<Hash256>>;

	/// **StorageMap** that maps a **variant of the *ProtoOwner* enum** to a **list of Proto-Fragment hashes (that have the aforementioned variant)**
	#[pallet::storage]
	pub type ProtosByOwner<T: Config> =
		StorageMap<_, Twox64Concat, ProtoOwner<T::AccountId>, Vec<Hash256>>;

	/// **StorageDoubleMap** that maps a **Proto-Fragment and a Clamor Account ID** to a **tuple that contains the Staked Amount (that was staked by the aforementioned Clamor Account ID) and the Block Number**
	// Staking management
	// (Amount staked, Last stake time)
	#[pallet::storage]
	pub type ProtoStakes<T: Config> = StorageDoubleMap<
		_,
		Identity,
		Hash256,
		Twox64Concat,
		T::AccountId,
		(T::Balance, T::BlockNumber),
	>;

	/// **StorageMap** that maps a **Clamor Account ID** to a **list of Proto-Fragments that was staked on by the aforementioned Clamor Account ID**
	#[pallet::storage]
	pub type AccountStakes<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<Hash256>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A Proto-Fragment was uploaded
		Uploaded { proto_hash: Hash256, cid: Vec<u8> },
		/// A Proto-Fragment was patched
		Patched { proto_hash: Hash256, cid: Vec<u8> },
		/// A Proto-Fragment metadata has changed
		MetadataChanged { proto_hash: Hash256, cid: Vec<u8> },
		/// A Proto-Fragment was detached
		Detached { proto_hash: Hash256, cid: Vec<u8> },
		/// A Proto-Fragment was transferred
		Transferred { proto_hash: Hash256, owner_id: T::AccountId },
		/// Stake was created
		Staked { proto_hash: Hash256, account_id: T::AccountId, balance: T::Balance },
		/// Stake was unlocked
		Unstaked { proto_hash: Hash256, account_id: T::AccountId, balance: T::Balance },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Systematic failure - those errors should not happen.
		SystematicFailure,
		/// Proto not found
		ProtoNotFound,
		/// Proto already uploaded
		ProtoExists,
		/// Already detached
		Detached,
		/// Not the owner of the proto
		Unauthorized,
		/// Not enough FRAG staked
		NotEnoughStaked,
		/// Stake not found
		StakeNotFound,
		/// Reference not found
		ReferenceNotFound,
		/// Not enough tokens to stake
		InsufficientBalance,
		/// Cannot unstake yet
		StakeLocked,
		/// Cannot find FRAG link to use as stake funds
		NoFragLink,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
	{
		/// **Upload** a **Proto-Fragment** onto the **Blockchain**.
		/// Furthermore, this function also indexes `data` in the Blockchain's Database and makes it available via bitswap (IPFS) directly from every chain node permanently.
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic function
		/// * `references` - **List of other Proto-Fragments** used to create the **Proto-Fragment**
		/// * `categories` - **Category type** of the **Proto-Fragment**
		/// * `tags` - **List of tags** to **tag** the **Proto-Fragment** **with**
		/// * `linked_asset` (*optional*) - An **asset that is linked with the Proto-Fragment** (e.g an ERC-721 Contract)
		/// * `include_cost` (*optional*) - **Price** of the **Proto-Fragment**. NOTE: If None, the **Proto-Fragment** *<u>can't be included</u>* into **other protos**
		/// * `data` - **Data** of the **Proto-Fragment**
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload() + (data.len() as u64 * <T as pallet::Config>::StorageBytesMultiplier::get()))]
		pub fn upload(
			origin: OriginFor<T>,
			// we store this in the state as well
			references: Vec<Hash256>,
			category: Categories,
			tags: Vec<Vec<u8>>,
			linked_asset: Option<LinkedAsset>,
			include_cost: Option<Compact<u64>>,
			// let data come last as we record this size in blocks db (storage chain)
			// and the offset is calculated like
			// https://github.com/paritytech/substrate/blob/a57bc4445a4e0bfd5c79c111add9d0db1a265507/client/db/src/lib.rs#L1678
			data: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			// hash the immutable data, this is also the unique proto id
			// to compose the V1 Cid add this prefix to the hash: (str "z" (base58
			// "0x0155a0e40220"))
			let proto_hash = blake2_256(&data);
			let data_len = data.len();

			// make sure the proto does not exist already!
			ensure!(!<Protos<T>>::contains_key(&proto_hash), Error::<T>::ProtoExists);

			// we need this to index transactions
			let extrinsic_index = <frame_system::Pallet<T>>::extrinsic_index()
				.ok_or(Error::<T>::SystematicFailure)?;

			// Check FRAG staking
			Self::check_staking_req(&references, &who)?;

			// ! Write STATE from now, ensure no errors from now...

			let owner = if let Some(link) = linked_asset {
				ProtoOwner::ExternalAsset(link)
			} else {
				ProtoOwner::User(who.clone())
			};

			let tags = tags
				.iter()
				.map(|s| {
					let tag_index = <Tags<T>>::get(s);
					if let Some(tag_index) = tag_index {
						<Compact<u64>>::from(tag_index)
					} else {
						let next_index = <TagsIndex<T>>::try_get().unwrap_or_default() + 1;
						<Tags<T>>::insert(s, next_index);
						// storing is dangerous inside a closure
						// but after this call we start storing..
						// so it's fine here
						<TagsIndex<T>>::put(next_index);
						<Compact<u64>>::from(next_index)
					}
				})
				.collect();

			// store in the state the proto
			let proto = Proto {
				block: current_block_number,
				patches: vec![],
				include_cost,
				creator: who.clone(),
				owner: owner.clone(),
				references,
				category: category.clone(),
				tags,
				metadata: BTreeMap::new(),
				accounts_info: AccountsInfo::default(),
			};

			// store proto
			<Protos<T>>::insert(proto_hash, proto);

			// store by category
			<ProtosByCategory<T>>::append(category, proto_hash);

			<ProtosByOwner<T>>::append(owner, proto_hash);

			// index immutable data for IPFS discovery
			transaction_index::index(extrinsic_index, data_len as u32, proto_hash);

			let cid = [&CID_PREFIX[..], &proto_hash[..]].concat();
			let cid = cid.to_base58();
			let cid = [&b"z"[..], cid.as_bytes()].concat();

			// also emit event
			Self::deposit_event(Event::Uploaded { proto_hash, cid });

			log::debug!("Uploaded proto: {:?}", proto_hash);

			Ok(())
		}

		/// **Patch** an **existing Proto-Fragment** (*by appending the hash of `data` to the Vector field `patches` of the existing Proto-Fragment's Struct Instance*)
		/// Furthermore, this function also indexes `data` in the Blockchain's Database and stores it in the IPFS
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic function
		/// * `proto_hash` - Existing Proto-Fragment's hash
		/// * `include_cost` (optional) -
		/// * `new_references` - **List of New Proto-Fragments** that was **used** to **create** the **patch**
		/// * `new_tags` - **List of Tags**, notice: it will replace previous tags if not None
		/// * `data` - **Data** of the **Proto-Fragment**
		#[pallet::weight(<T as pallet::Config>::WeightInfo::patch() + data.len() as u64 * <T as pallet::Config>::StorageBytesMultiplier::get())]
		pub fn patch(
			origin: OriginFor<T>,
			// proto hash we want to patch
			proto_hash: Hash256,
			include_cost: Option<Compact<u64>>,
			new_references: Vec<Hash256>,
			new_tags: Option<Vec<Vec<u8>>>,
			// data we want to patch last because of the way we store blocks (storage chain)
			data: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(&proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			match proto.owner {
				ProtoOwner::User(owner) => ensure!(owner == who, Error::<T>::Unauthorized),
				ProtoOwner::ExternalAsset(_ext_asset) =>
				// We don't allow updating external assets
				{
					ensure!(false, Error::<T>::Unauthorized)
				},
			};

			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			let data_hash = blake2_256(&data);

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			// we need this to index transactions
			let extrinsic_index = <frame_system::Pallet<T>>::extrinsic_index()
				.ok_or(Error::<T>::SystematicFailure)?;

			// Check FRAG staking
			Self::check_staking_req(&new_references, &who)?;

			// Write STATE from now, ensure no errors from now...

			<Protos<T>>::mutate(&proto_hash, |proto| {
				let proto = proto.as_mut().unwrap();

				// Add a data patch if not empty
				if !data.is_empty() {
					// No failures from here on out
					proto.patches.push(ProtoPatch {
						block: current_block_number,
						data_hash,
						references: new_references,
					});
					// index mutable data for IPFS discovery as well
					transaction_index::index(extrinsic_index, data.len() as u32, data_hash);
				}

				// Overwrite include cost
				proto.include_cost = include_cost;

				// Replace previous tags if not None
				if let Some(new_tags) = new_tags {
					let tags = new_tags
						.iter()
						.map(|s| {
							let tag_index = <Tags<T>>::get(s);
							if let Some(tag_index) = tag_index {
								<Compact<u64>>::from(tag_index)
							} else {
								let next_index = <TagsIndex<T>>::try_get().unwrap_or_default() + 1;
								<Tags<T>>::insert(s, next_index);
								// storing is dangerous inside a closure
								// but after this call we start storing..
								// so it's fine here
								<TagsIndex<T>>::put(next_index);
								<Compact<u64>>::from(next_index)
							}
						})
						.collect();

					proto.tags = tags;
				}
			});

			let cid = [&CID_PREFIX[..], &data_hash[..]].concat();
			let cid = cid.to_base58();
			let cid = [&b"z"[..], cid.as_bytes()].concat();

			// also emit event
			Self::deposit_event(Event::Patched { proto_hash, cid });

			log::debug!("Updated proto: {:?}", proto_hash);

			Ok(())
		}

		/// **Transfer** the **ownership** of a **Proto-Fragment** from **`origin`** to **`new_owner`**
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic function
		/// * `proto_hash` - The **hash of the data of the Proto-Fragment** to **transfer**
		/// * `new_owner` - The **Account ID** to **transfer the Proto-Fragment to**
		#[pallet::weight(<T as pallet::Config>::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			proto_hash: Hash256,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// make sure the proto exists
			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(&proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			// make sure the caller is the owner
			match proto.owner.clone() {
				ProtoOwner::User(owner) => ensure!(owner == who, Error::<T>::Unauthorized),
				ProtoOwner::ExternalAsset(_ext_asset) =>
				// We don't allow updating external assets
				{
					ensure!(false, Error::<T>::Unauthorized)
				},
			};

			// make sure the proto is not detached
			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			// collect new owner
			let new_owner_s = ProtoOwner::User(new_owner.clone());

			// WRITING STATE FROM NOW

			// remove proto from old owner
			<ProtosByOwner<T>>::mutate(proto.owner, |proto_by_owner| {
				if let Some(list) = proto_by_owner {
					list.retain(|current_hash| proto_hash != *current_hash);
				}
			});

			// add proto to new owner
			<ProtosByOwner<T>>::append(new_owner_s.clone(), proto_hash);

			// update proto
			<Protos<T>>::mutate(&proto_hash, |proto| {
				let proto = proto.as_mut().unwrap();
				proto.owner = new_owner_s;
			});

			// emit event
			Self::deposit_event(Event::Transferred { proto_hash, owner_id: new_owner });

			Ok(())
		}

		/// **Alters** the **metadata** of a **Proto-Fragment** (whose hash is `proto_hash`) by **adding or modifying a key-value pair** (`metadata_key.clone`,`blake2_256(&data.encode())`) to the **BTreeMap field `metadata`** of the **existing Proto-Fragment's Struct Instance**.
		/// Furthermore, this function also indexes `data` in the Blockchain's Database and stores it in the IPFS
		/// To successfully patch a Proto-Fragment, the `auth` provided must be valid. Otherwise, an error is returned (问Gio - what happened to auth?)
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic / dispatchable function
		/// * `proto_hash` - The **hash of the Proto-Fragment**
		/// * `metadata_key` - The key (of the key-value pair) that is added in the BTreeMap field `metadata` of the existing Proto-Fragment's Struct Instance
		/// * `data` - The hash of `data` is used as the value (of the key-value pair) that is added in the BTreeMap field `metadata` of the existing Proto-Fragment's Struct Instance
		#[pallet::weight(<T as pallet::Config>::WeightInfo::patch() + (data.len() as u64 * <T as pallet::Config>::StorageBytesMultiplier::get()))]
		pub fn set_metadata(
			origin: OriginFor<T>,
			// proto hash we want to update
			proto_hash: Hash256,
			// Think of "Vec<u8>" as String (something to do with WASM - that's why we use Vec<u8>)
			metadata_key: Vec<u8>,
			// data we want to update last because of the way we store blocks (storage chain)
			data: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(&proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			match proto.owner {
				ProtoOwner::User(owner) => ensure!(owner == who, Error::<T>::Unauthorized),
				ProtoOwner::ExternalAsset(_ext_asset) =>
				// We don't allow updating external assets
				{
					ensure!(false, Error::<T>::Unauthorized)
				},
			};

			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			let data_hash = blake2_256(&data);

			// we need this to index transactions
			let extrinsic_index = <frame_system::Pallet<T>>::extrinsic_index()
				.ok_or(Error::<T>::SystematicFailure)?;

			let metadata_key_index = {
				let index = <MetaKeys<T>>::get(metadata_key.clone());
				if let Some(index) = index {
					<Compact<u64>>::from(index)
				} else {
					let next_index = <MetaKeysIndex<T>>::try_get().unwrap_or_default() + 1;
					<MetaKeys<T>>::insert(metadata_key.clone(), next_index);
					// storing is dangerous inside a closure
					// but after this call we start storing..
					// so it's fine here
					<MetaKeysIndex<T>>::put(next_index);
					<Compact<u64>>::from(next_index)
				}
			};

			// Write STATE from now, ensure no errors from now...

			<Protos<T>>::mutate(&proto_hash, |proto| {
				let proto = proto.as_mut().unwrap();
				// update metadata
				proto.metadata.insert(metadata_key_index, data_hash);
			});

			// index data
			transaction_index::index(extrinsic_index, data.len() as u32, data_hash);

			// also emit event
			Self::deposit_event(Event::MetadataChanged { proto_hash, cid: metadata_key.clone() });

			log::debug!("Added metadata to proto: {:x?} with key: {:x?}", proto_hash, metadata_key);

			Ok(())
		}

		/// **Detach** a **Proto-Fragment** from **this blockchain** to an **external blockchain** by ***initiating*** an **event** that **includes a signature**. (NC)
		/// The **owner of this Proto-Fragment** can then **attach this Proto-Fragment** to the **external blockchain** by **using the aforementioned signature**.
		///
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic function
		/// * `proto_hash` - **Hash of the Proto-Fragment** to **detach**
		/// * `target_chain` - **External Blockchain** that we **want to attach the **Proto-Fragment into**
		/// * `target_account` - **Public account address** of the **blockchain `target_chain`** that we **want to detach the Proto-Fragment into**
		#[pallet::weight(<T as pallet::Config>::WeightInfo::detach())]
		pub fn detach(
			origin: OriginFor<T>,
			proto_hash: Hash256,
			target_chain: SupportedChains,
			target_account: Vec<u8>, // an eth address or so
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// make sure the proto exists
			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(&proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			match proto.owner {
				ProtoOwner::User(owner) => ensure!(owner == who, Error::<T>::Unauthorized),
				ProtoOwner::ExternalAsset(_ext_asset) =>
				// We don't allow detaching external assets
				{
					ensure!(false, Error::<T>::Unauthorized)
				},
			};

			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			<DetachRequests<T>>::mutate(|requests| {
				requests.push(DetachRequest { hash: proto_hash, target_chain, target_account });
			});

			Ok(())
		}

		/// Stake FRAG tokens on a Proto-Fragment
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic function
		/// * `proto_hash` - **Hash of the Proto-Fragment** to **stake on**
		/// * `amount` - **Amount of FRAG tokens** to **stake**
		#[pallet::weight(50_000)]
		pub fn stake(
			origin: OriginFor<T>,
			proto_hash: Hash256,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			let who = get_locked_frag_account(&who).map_err(|_| Error::<T>::SystematicFailure)?;

			// make sure the proto exists
			ensure!(<Protos<T>>::contains_key(&proto_hash), Error::<T>::ProtoNotFound);

			// make sure user has enough FRAG
			let account = <pallet_accounts::EVMLinks<T>>::get(&who.clone())
				.ok_or_else(|| Error::<T>::NoFragLink)?;
			let eth_lock = <pallet_accounts::EthLockedFrag<T>>::get(&account)
				.ok_or_else(|| Error::<T>::NoFragLink)?; // Amount of FRAG locked by Ethereum Account `account`
			let balance = eth_lock.amount
				- <pallet_accounts::FragUsage<T>>::get(&who.clone())
					.ok_or_else(|| Error::<T>::NoFragLink)?; // Balance = Amount of FRAG locked - Amount of FRAG already staked
			ensure!(balance >= amount, Error::<T>::InsufficientBalance);

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			// ! from now we write...

			<pallet_accounts::FragUsage<T>>::mutate(&who, |usage| {
				// Add `amount` to the FragUsage of `who`
				usage.as_mut().unwrap().saturating_add(amount);
			});

			// take record of the stake
			<ProtoStakes<T>>::insert(proto_hash, &who, (amount, current_block_number));
			<AccountStakes<T>>::append(who.clone(), proto_hash.clone());

			// also emit event
			Self::deposit_event(Event::Staked { proto_hash, account_id: who, balance: amount }); // 问Gio

			Ok(())
		}

		/// Unstake the FRAG tokens that were staked on a Proto-Fragment by `origin`
		///
		/// # Arguments
		///
		/// * `origin` - The origin of the extrinsic function
		/// * `proto_hash` - **Hash of the Proto-Fragment** on which the FRAG tokens were staked on
		#[pallet::weight(50_000)]
		pub fn unstake(origin: OriginFor<T>, proto_hash: Hash256) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			// make sure the proto exists
			ensure!(<Protos<T>>::contains_key(&proto_hash), Error::<T>::ProtoNotFound);

			// make sure user has enough FRAG
			let stake =
				<ProtoStakes<T>>::get(&proto_hash, &who).ok_or(Error::<T>::StakeNotFound)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(
				current_block_number > (stake.1 + T::StakeLockupPeriod::get().saturated_into()), 
				Error::<T>::StakeLocked
			);

			// ! from now we write...

			<pallet_accounts::FragUsage<T>>::mutate(&who, |usage| {
				usage.as_mut().unwrap().saturating_sub(stake.0); // Reduce the Frag usage of `who` by `stake.0`
			});

			// take record of the unstake
			<ProtoStakes<T>>::remove(proto_hash, &who);
			<AccountStakes<T>>::mutate(who.clone(), |stakes| {
				if let Some(stakes) = stakes {
					stakes.retain(|h| h != &proto_hash); // Retain all Proto-Fragments except `proto_hash`
				}
			});

			// also emit event
			Self::deposit_event(Event::Unstaked { proto_hash, account_id: who, balance: stake.0 });

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// During the block finalization phase,
		/// clear all the staking-related (of FRAG Token) Storage Items of any information regarding
		/// the list of Clamor Account IDs in `PendingUnlocks`. And then subsequently, clear `PendingUnlocks`.
		fn on_finalize(_n: T::BlockNumber) {
			// drain unlinks
			let unlinks = <pallet_accounts::PendingUnlinks<T>>::take();
			for unlink in unlinks {
				// take emptying the storage
				let stakes = <AccountStakes<T>>::take(unlink.clone());
				if let Some(stakes) = stakes {
					for stake in stakes {
						<ProtoStakes<T>>::remove(stake, &unlink);
					}
				}
			}
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
	{
		fn check_staking_req(references: &[Hash256], who: &T::AccountId) -> DispatchResult {
			// Check FRAG staking
			// TODO this is not tested properly
			for reference in references.iter() {
				let proto = <Protos<T>>::get(reference);
				if let Some(proto) = proto {
					let owner = match proto.owner {
						ProtoOwner::User(owner) => Some(owner),
						_ => None,
					};

					if let Some(owner) = owner {
						if owner == *who {
							// owner can include freely
							continue
						}
					}

					let cost = proto.include_cost;
					if let Some(cost) = cost {
						let cost: u64 = cost.into();
						if cost == 0 {
							// free
							continue
						}
						let cost: T::Balance = cost.saturated_into();
						let stake = <ProtoStakes<T>>::get(reference, who.clone());
						if let Some(stake) = stake {
							ensure!(stake.0 >= cost, Error::<T>::NotEnoughStaked);
						} else {
							// Stake not found
							return Err(Error::<T>::StakeNotFound.into())
						}
					// OK to include, just continue
					} else {
						return Err(Error::<T>::Unauthorized.into())
					}
				} else {
					// Proto not found
					return Err(Error::<T>::ReferenceNotFound.into())
				}
			}
			Ok(())
		}

		fn filter_proto(
			proto_id: &Hash256,
			tags: &[Vec<u8>],
			categories: &[Categories],
			avail: Option<bool>,
		) -> bool {
			if let Some(struct_proto) = <Protos<T>>::get(proto_id) {
				if let Some(avail) = avail {
					if avail && struct_proto.include_cost.is_none() {
						return false
					} else if !avail && struct_proto.include_cost.is_some() {
						return false
					}
				}

				if categories.len() == 0
				// Use any here to match any category towards proto
					|| categories.into_iter().any(|cat| *cat == struct_proto.category)
				{
					// Use all here to match all tags always
					if tags.len() == 0 {
						true
					} else {
						tags.into_iter().all(|tag| {
							let tag_idx = <Tags<T>>::get(tag);
							if let Some(tag_idx) = tag_idx {
								struct_proto.tags.contains(&Compact::from(tag_idx))
							} else {
								false
							}
						})
					}
				} else {
					false
				}
			} else {
				false
			}
		}

		/// **Query** and **Return** **Proto-Fragment(s)** based on **`params`**. The **return type** is a **JSON string**
		/// Furthermore, this function also indexes `data` in the Blockchain's Database and makes it available via bitswap (IPFS) directly from every chain node permanently.
		///
		/// # Arguments
		///
		/// * `params` - A ***GetProtosParams* struct**
		pub fn get_protos(
			params: GetProtosParams<T::AccountId, Vec<u8>>,
		) -> Result<Vec<u8>, Vec<u8>> {
			let mut map = Map::new();

			let list_protos_final: Vec<Hash256> = if let Some(owner) = params.owner {
				// `owner` exists
				if let Some(list_protos_owner) =
					<ProtosByOwner<T>>::get(ProtoOwner::<T::AccountId>::User(owner))
				{
					// `owner` exists in `ProtosByOwner`
					if params.desc {
						// Sort in descending order
						list_protos_owner
							.into_iter()
							.rev()
							.filter(|proto_id| {
								Self::filter_proto(
									proto_id,
									&params.tags,
									&params.categories,
									params.available,
								)
							})
							.skip(params.from as usize)
							.take(params.limit as usize)
							.collect::<Vec<Hash256>>()
					} else {
						// Sort in ascending order
						list_protos_owner
							.into_iter()
							.filter(|proto_id| {
								Self::filter_proto(
									proto_id,
									&params.tags,
									&params.categories,
									params.available,
								)
							})
							.skip(params.from as usize)
							.take(params.limit as usize)
							.collect::<Vec<Hash256>>()
					}
				} else {
					// `owner` doesn't exist in `ProtosByOwner`
					return Err("Owner not found".into())
				}
			} else {
				// Notice this wastes time and memory and needs a better implementation
				let mut flat = Vec::<Hash256>::new();

				let cats = <ProtosByCategory<T>>::iter_keys().collect::<Vec<Categories>>();
				let cats: Vec<Categories> =
					if params.desc { cats.iter().rev().map(|x| x.clone()).collect() } else { cats };

				for category in cats {
					if params.categories.len() != 0 && !params.categories.contains(&category) {
						continue
					}

					let protos = <ProtosByCategory<T>>::get(category);
					if let Some(protos) = protos {
						let collection: Vec<Hash256> = if params.desc {
							// Sort in descending order
							protos
								.into_iter()
								.rev()
								.filter(|proto_id| {
									Self::filter_proto(
										proto_id,
										&params.tags,
										&params.categories,
										params.available,
									)
								})
								.collect()
						} else {
							// Sort in ascending order
							protos
								.into_iter()
								.filter(|proto_id| {
									Self::filter_proto(
										proto_id,
										&params.tags,
										&params.categories,
										params.available,
									)
								})
								.collect()
						};
						flat.extend(collection);
					}
				}

				flat.iter()
					.skip(params.from as usize)
					.take(params.limit as usize)
					.map(|x| *x)
					.collect()
			};

			for proto_id in list_protos_final.into_iter() {
				map.insert(hex::encode(proto_id), Value::Object(Map::new()));
			}

			if params.return_owners || !params.metadata_keys.is_empty() {
				for (proto_id, map_proto) in map.iter_mut() {
					let array_proto_id: Hash256 = if let Ok(array_proto_id) = hex::decode(proto_id)
					{
						if let Ok(array_proto_id) = array_proto_id.try_into() {
							array_proto_id
						} else {
							return Err("Failed to convert proto_id to Hash256".into())
						}
					} else {
						return Err("Failed to decode proto_id".into())
					};

					let (owner, map_metadata, include_cost) =
						if let Some(proto) = <Protos<T>>::get(array_proto_id) {
							(proto.owner, proto.metadata, proto.include_cost)
						} else {
							return Err("Failed to get proto".into())
						};

					let map_proto = match map_proto {
						Value::Object(map_proto) => map_proto,
						_ => return Err("Failed to get map_proto".into()),
					};

					if let Some(include_cost) = include_cost {
						let n: u64 = include_cost.into();
						(*map_proto).insert(String::from("include_cost"), Value::Number(n.into()));
					} else {
						(*map_proto).insert(String::from("include_cost"), Value::Null);
					}

					if params.return_owners {
						let owner = match owner {
							ProtoOwner::User(account_id) => json!({
								"type": "internal",
								"value": hex::encode(account_id)
							}),
							ProtoOwner::ExternalAsset(linked_asset) => {
								let value = match linked_asset {
									LinkedAsset::Erc721(contract, token_id, source) => {
										let chain_id = match source {
											LinkSource::Evm(_sig, _block, chain_id) => chain_id,
										};
										json!({
											"type": "erc721",
											"value": {
												"contract": format!("0x{:x}", contract),
												"token_id": format!("0x{:x}", token_id),
												"chain_id": format!("0x{:x}", chain_id)
											}
										})
									},
								};
								json!({
									"type": "external",
									"value": value,
								})
							},
						};

						(*map_proto).insert(String::from("owner"), owner);
					}

					if !params.metadata_keys.is_empty() {
						for metadata_key in params.metadata_keys.iter() {
							let metadata_key_index = <MetaKeys<T>>::get(metadata_key.clone());
							let metadata_value = if let Some(metadata_key_index) =
								metadata_key_index
							{
								let metadata_key_index = <Compact<u64>>::from(metadata_key_index);

								if let Some(data_hash) = map_metadata.get(&metadata_key_index) {
									Value::String(hex::encode(data_hash))
								} else {
									Value::Null
								}
							} else {
								Value::Null
							};

							if let Ok(key) = String::from_utf8(metadata_key.clone()) {
								(*map_proto).insert(key, metadata_value);
							}
						}
					}
				}
			}

			let result = json!(map).to_string();

			Ok(result.into_bytes())
		}
	}
}

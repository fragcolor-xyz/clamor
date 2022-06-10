#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod weights;

use codec::{Compact, Decode, Encode};
pub use pallet::*;
use sp_clamor::{Hash128, Hash256};
use sp_io::{
	hashing::{blake2_128, blake2_256},
	transaction_index,
};
use sp_std::vec::Vec;
pub use weights::WeightInfo;

use protos::permissions::FragmentPerms;

use frame_support::{dispatch::DispatchResult, PalletId};
use sp_runtime::traits::{AccountIdConversion, StaticLookup};

use frame_support::traits::{tokens::fungibles::Transfer, Currency, ExistenceRequirement};
use sp_runtime::SaturatedConversion;

const PALLET_ID: PalletId = PalletId(*b"fragment");

type Unit = u64;

#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct FragmentMetadata<TFungibleAsset> {
	pub name: Vec<u8>,
	pub currency: Option<TFungibleAsset>, // Where None is NOVA
}

/// Struct of a Fragment Class
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct FragmentClass<TFungibleAsset, TAccountId> {
	/// The Proto-Fragment that was used to create this Fragment Class
	pub proto_hash: Hash256,
	/// The metadata of the Fragment Class
	pub metadata: FragmentMetadata<TFungibleAsset>,
	/// The next owner permissions
	pub permissions: FragmentPerms,
	/// If Fragments must contain unique data when created (injected by buyers, validated by the system)
	pub unique: bool,
	/// If scarce, the max supply of the Fragment
	pub max_supply: Option<Compact<Unit>>,
	/// The creator of this class
	pub creator: TAccountId,
}

#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct InstanceData<TBlockNum> {
	/// Next owner permissions, owners can change those if they want to more restrictive ones, never more permissive
	pub permissions: FragmentPerms,
	/// The block number when the item was created
	pub created_at: TBlockNum,
	/// Custom data, if unique, this is the hash of the data that can be fetched using bitswap directly on our nodes
	pub custom_data: Option<Hash256>,
}

#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub struct PublishingData<TBlockNum> {
	pub price: Compact<u128>,
	pub units_left: Option<Compact<Unit>>,
	pub expiration: Option<TBlockNum>,
}

#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Debug, PartialEq)]
pub enum FragmentBuyOptions {
	UniqueData(Vec<u8>),
	Quantity(u64),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, Twox64Concat};
	use frame_system::pallet_prelude::*;
	use pallet_detach::DetachedHashes;
	use pallet_protos::{Proto, ProtoOwner, Protos};

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_protos::Config + pallet_assets::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// proto-hash to fragment-hash-sequence
	/// Storage Map that keeps track of the number of Fragments that were created using a Proto-Fragment.
	/// The key is the hash of the Proto-Fragment, and the value is the list of hash of the Fragments
	#[pallet::storage]
	pub type Proto2Fragments<T: Config> = StorageMap<_, Identity, Hash256, Vec<Hash128>>;

	// fragment-hash to fragment-data
	/// Storage Map of Fragments where the key is the hash of the concatenation of its corresponding Proto-Fragment and the name of the Fragment, and the value is the Fragment struct of the Fragment
	#[pallet::storage]
	pub type Classes<T: Config> =
		StorageMap<_, Identity, Hash128, FragmentClass<T::AssetId, T::AccountId>>;

	#[pallet::storage]
	pub type Publishing<T: Config> =
		StorageMap<_, Identity, Hash128, PublishingData<T::BlockNumber>>;

	#[pallet::storage]
	pub type EditionsCount<T: Config> = StorageMap<_, Identity, Hash128, u64>;

	#[pallet::storage]
	pub type CopiesCount<T: Config> = StorageMap<_, Twox64Concat, (Hash128, Compact<u64>), u64>;

	#[pallet::storage]
	pub type Fragments<T: Config> = StorageNMap<
		_,
		(
			storage::Key<Identity, Hash128>,
			// Editions
			storage::Key<Twox64Concat, Compact<Unit>>,
			// Copies
			storage::Key<Twox64Concat, Compact<Unit>>,
		),
		InstanceData<T::BlockNumber>,
	>;

	#[pallet::storage]
	pub type Owners<T: Config> = StorageDoubleMap<
		_,
		Identity,
		Hash128,
		Twox64Concat,
		T::AccountId,
		Vec<(Compact<Unit>, Compact<Unit>)>,
	>;

	#[pallet::storage]
	pub type Inventory<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Identity,
		Hash128,
		Vec<(Compact<Unit>, Compact<Unit>)>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New class created by account, class hash
		ClassCreated(Hash128),
		/// Fragment sale has been opened
		Publishing(Hash128),
		/// Fragment sale has been closed
		Unpublishing(Hash128),
		/// Inventory item has been added to account
		InventoryAdded(T::AccountId, Hash128, (Unit, Unit)),
		/// Inventory item has removed added from account
		InventoryRemoved(T::AccountId, Hash128, (Unit, Unit)),
		/// Inventory has been updated
		InventoryUpdated(T::AccountId, Hash128, (Unit, Unit)),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Proto not found
		ProtoNotFound,
		/// Proto owner not found
		ProtoOwnerNotFound,
		/// No Permission
		NoPermission,
		/// Already detached
		Detached,
		/// Already exist
		AlreadyExist,
		/// Not found
		NotFound,
		/// Sale has expired
		Expired,
		/// Insufficient funds
		InsufficientBalance,
		/// Fragment sale sold out
		SoldOut,
		/// Sale already open
		SaleAlreadyOpen,
		/// Max supply reached
		MaxSupplyReached,
		/// Params not valid
		ParamsNotValid,
		/// This should not really happen
		SystematicFailure,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a Fragment Class using an existing Proto (only the owner of the Proto can call this function and create a new Fragment Class based on the Proto)
		///
		/// # Arguments
		/// * `origin` - The origin of the extrinsic/dispatchable function.
		/// * `proto_hash` - The hash of the existing Proto-Fragment
		/// * `metadata` - The metadata (name, external url etc.) of the Fragment that is going to be created
		/// * `permissions` - The permissions that the next owner of the Fragment will have
		/// * `unique` - If the Fragments generated should be unique (only one Fragment can exist with the same exact data)
		/// * `max_supply` (optional) - if scarce, the maximum amount of items that can be ever created (doesn't apply to copies if the item can be copied!) of this type
		#[pallet::weight(<T as Config>::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			proto_hash: Hash256,
			metadata: FragmentMetadata<T::AssetId>,
			permissions: FragmentPerms,
			unique: bool,
			max_supply: Option<Unit>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			let proto_owner: T::AccountId = match proto.owner {
				ProtoOwner::User(owner) => Ok(owner),
				_ => Err(Error::<T>::ProtoOwnerNotFound),
			}?;

			ensure!(who == proto_owner, Error::<T>::NoPermission);

			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			let hash = blake2_128(
				&[&proto_hash[..], &metadata.name.encode(), &metadata.currency.encode()].concat(),
			);

			ensure!(!<Classes<T>>::contains_key(&hash), Error::<T>::AlreadyExist);

			// ! Writing

			// create vault account
			// we need an existential amount deposit to be able to create the vault account
			let vault = Self::get_vault_id(hash);
			let min_balance =
				<pallet_balances::Pallet<T> as Currency<T::AccountId>>::minimum_balance();
			let _ = <pallet_balances::Pallet<T> as Currency<T::AccountId>>::deposit_creating(
				&vault,
				min_balance,
			);

			let fragment_data = FragmentClass {
				proto_hash,
				metadata,
				permissions,
				unique,
				max_supply: max_supply.map(|x| Compact(x)),
				creator: who.clone(),
			};
			<Classes<T>>::insert(&hash, fragment_data);

			Proto2Fragments::<T>::append(&proto_hash, hash);

			Self::deposit_event(Event::ClassCreated(hash));
			Ok(())
		}

		#[pallet::weight(50_000)]
		pub fn publish(
			origin: OriginFor<T>,
			fragment_hash: Hash128,
			price: u128,
			quantity: Option<Unit>,
			expires: Option<T::BlockNumber>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let proto_hash =
				<Classes<T>>::get(fragment_hash).ok_or(Error::<T>::NotFound)?.proto_hash;
			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			let proto_owner: T::AccountId = match proto.owner {
				ProtoOwner::User(owner) => Ok(owner),
				_ => Err(Error::<T>::ProtoOwnerNotFound),
			}?;

			ensure!(who == proto_owner, Error::<T>::NoPermission);

			// TO REVIEW
			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			ensure!(!<Publishing<T>>::contains_key(&fragment_hash), Error::<T>::SaleAlreadyOpen);

			let fragment_data = <Classes<T>>::get(fragment_hash).ok_or(Error::<T>::NotFound)?;

			if let Some(max_supply) = fragment_data.max_supply {
				let max: Unit = max_supply.into();
				let existing: Unit =
					<EditionsCount<T>>::get(&fragment_hash).unwrap_or(0);
				let left = max.saturating_sub(existing);
				if let Some(quantity) = quantity {
					let quantity: Unit = quantity.into();
					ensure!(quantity <= left, Error::<T>::MaxSupplyReached);
				} else {
					return Err(Error::<T>::ParamsNotValid.into());
				}
				if left == 0 {
					return Err(Error::<T>::MaxSupplyReached.into());
				}
			}

			// ! Writing

			<Publishing<T>>::insert(
				fragment_hash,
				PublishingData {
					price: Compact(price),
					units_left: quantity.map(|x| Compact(x)),
					expiration: expires,
				},
			);

			Self::deposit_event(Event::Publishing(fragment_hash));

			Ok(())
		}

		#[pallet::weight(50_000)]
		pub fn unpublish(origin: OriginFor<T>, fragment_hash: Hash128) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let proto_hash =
				<Classes<T>>::get(fragment_hash).ok_or(Error::<T>::NotFound)?.proto_hash;
			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			let proto_owner: T::AccountId = match proto.owner {
				ProtoOwner::User(owner) => Ok(owner),
				_ => Err(Error::<T>::ProtoOwnerNotFound),
			}?;

			ensure!(who == proto_owner, Error::<T>::NoPermission);

			// TO REVIEW
			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			// ! Writing

			<Publishing<T>>::remove(&fragment_hash);

			Self::deposit_event(Event::Unpublishing(fragment_hash));

			Ok(())
		}

		/// Proto owner can mint fragments (compatible with supply requirements)
		#[pallet::weight(50_000)]
		pub fn mint(
			origin: OriginFor<T>,
			fragment_hash: Hash128,
			options: FragmentBuyOptions,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			let proto_hash =
				<Classes<T>>::get(fragment_hash).ok_or(Error::<T>::NotFound)?.proto_hash;
			let proto: Proto<T::AccountId, T::BlockNumber> =
				<Protos<T>>::get(proto_hash).ok_or(Error::<T>::ProtoNotFound)?;

			let proto_owner: T::AccountId = match proto.owner {
				ProtoOwner::User(owner) => Ok(owner),
				_ => Err(Error::<T>::ProtoOwnerNotFound),
			}?;

			ensure!(who == proto_owner, Error::<T>::NoPermission);

			// TO REVIEW
			ensure!(!<DetachedHashes<T>>::contains_key(&proto_hash), Error::<T>::Detached);

			let quantity = match options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};

			// ! Writing

			Self::mint_fragments(
				&who,
				&fragment_hash,
				None,
				&options,
				quantity,
				current_block_number,
			)
		}

		/// When a sale is open users can buy fragments
		#[pallet::weight(50_000)]
		pub fn buy(
			origin: OriginFor<T>,
			fragment_hash: Hash128,
			options: FragmentBuyOptions,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			let sale = <Publishing<T>>::get(&fragment_hash).ok_or(Error::<T>::NotFound)?;
			if let Some(expiration) = sale.expiration {
				ensure!(current_block_number < expiration, Error::<T>::Expired);
			}

			if let Some(units_left) = sale.units_left {
				ensure!(units_left > Compact(0), Error::<T>::SoldOut);
			}

			let price: u128 = sale.price.into();

			let fragment_data = <Classes<T>>::get(fragment_hash).ok_or(Error::<T>::NotFound)?;

			let vault = &Self::get_vault_id(fragment_hash);

			let quantity = match options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};

			let price = price.saturating_mul(quantity as u128);

			// ! Writing if successful

			if let Some(currency) = fragment_data.metadata.currency {
				<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
					currency,
					&who,
					&Self::get_vault_id(fragment_hash),
					price.saturated_into(),
					true,
				)
				.map_err(|_| Error::<T>::InsufficientBalance)?;
			} else {
				<pallet_balances::Pallet<T> as Currency<T::AccountId>>::transfer(
					&who,
					&vault,
					price.saturated_into(),
					ExistenceRequirement::KeepAlive,
				)
				.map_err(|_| Error::<T>::InsufficientBalance)?;
			}

			// ! We should be successful here

			Self::mint_fragments(
				&who,
				&fragment_hash,
				Some(&sale),
				&options,
				quantity,
				current_block_number,
			)
		}

		#[pallet::weight(50_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			class: Hash128,
			edition: Unit,
			copy: Unit,
			to: <T::Lookup as StaticLookup>::Source,
			new_permissions: Option<FragmentPerms>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let item_data = <Fragments<T>>::get((class, Compact(edition), Compact(copy)))
				.ok_or(Error::<T>::NotFound)?;

			// Only the owner of this fragment can set it for sale
			let ids = <Inventory<T>>::get(who.clone(), class).ok_or(Error::<T>::NotFound)?;

			ensure!(ids.contains(&(Compact(edition), Compact(copy))), Error::<T>::NoPermission);

			// first of all make sure the item can be transferred
			ensure!(
				(item_data.permissions & FragmentPerms::TRANSFER) == FragmentPerms::TRANSFER,
				Error::<T>::NoPermission
			);

			let perms = if let Some(new_perms) = new_permissions {
				// ensure we only allow more restrictive permissions
				if (item_data.permissions & FragmentPerms::EDIT) != FragmentPerms::EDIT {
					ensure!(
						(new_perms & FragmentPerms::EDIT) != FragmentPerms::EDIT,
						Error::<T>::NoPermission
					);
				}
				if (item_data.permissions & FragmentPerms::COPY) != FragmentPerms::COPY {
					ensure!(
						(new_perms & FragmentPerms::COPY) != FragmentPerms::COPY,
						Error::<T>::NoPermission
					);
				}
				if (item_data.permissions & FragmentPerms::TRANSFER) != FragmentPerms::TRANSFER {
					ensure!(
						(new_perms & FragmentPerms::TRANSFER) != FragmentPerms::TRANSFER,
						Error::<T>::NoPermission
					);
				}
				new_perms
			} else {
				item_data.permissions
			};

			let to = T::Lookup::lookup(to)?;

			// now we take two different paths if item can be copied or not
			if (item_data.permissions & FragmentPerms::COPY) == FragmentPerms::COPY {
				// we will copy the item to the new account
				// TODO
			} else {
				// we will remove from this account to give to new account
				<Owners<T>>::mutate(class, who.clone(), |ids| {
					if let Some(ids) = ids {
						ids.retain(|cid| *cid != (Compact(edition), Compact(copy)))
					}
				});

				<Inventory<T>>::mutate(who.clone(), class, |ids| {
					if let Some(ids) = ids {
						ids.retain(|cid| *cid != (Compact(edition), Compact(copy)))
					}
				});

				Self::deposit_event(Event::InventoryRemoved(who.clone(), class, (edition, copy)));

				<Owners<T>>::append(class, to.clone(), (Compact(edition), Compact(copy)));

				<Inventory<T>>::append(to.clone(), class, (Compact(edition), Compact(copy)));

				Self::deposit_event(Event::InventoryAdded(to, class, (edition, copy)));

				// finally fix permissions that might have changed
				<Fragments<T>>::mutate((class, Compact(edition), Compact(copy)), |item_data| {
					if let Some(item_data) = item_data {
						item_data.permissions = perms;
					}
				});
			}

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_vault_id(class_hash: Hash128) -> T::AccountId {
		PALLET_ID.into_sub_account_truncating(class_hash)
	}

	pub fn get_fragment_account_id(class_hash: Hash128, id: Unit) -> T::AccountId {
		PALLET_ID.into_sub_account_truncating((class_hash, id))
	}

	pub fn mint_fragments(
		to: &T::AccountId,
		fragment_hash: &Hash128,
		sale: Option<&PublishingData<T::BlockNumber>>,
		options: &FragmentBuyOptions,
		quantity: u64,
		current_block_number: T::BlockNumber,
	) -> DispatchResult {
		let data = match options {
			FragmentBuyOptions::UniqueData(data) => {
				let data_hash = blake2_256(&data);
				let data_len = data.len();

				// we need this to index transactions
				let extrinsic_index = <frame_system::Pallet<T>>::extrinsic_index()
					.ok_or(Error::<T>::SystematicFailure)?;

				// index immutable data for IPFS discovery
				transaction_index::index(extrinsic_index, data_len as u32, data_hash);

				Some(data_hash)
			},
			_ => None,
		};

		let existing: Unit = <EditionsCount<T>>::get(&fragment_hash).unwrap_or(0);

		if let Some(sale) = sale {
			// if limited amount let's reduce the amount of units left
			if let Some(units_left) = sale.units_left {
				<Publishing<T>>::mutate(&*fragment_hash, |sale| {
					if let Some(sale) = sale {
						let left: Unit = units_left.into();
						sale.units_left = Some(Compact(left - quantity));
					}
				});
			}
		} else {
			// We still don't wanna go over supply limit
			let fragment_data = <Classes<T>>::get(fragment_hash).ok_or(Error::<T>::NotFound)?;

			if let Some(max_supply) = fragment_data.max_supply {
				let max: Unit = max_supply.into();
				let left = max.saturating_sub(existing);
				if quantity <= left {
					return Err(Error::<T>::MaxSupplyReached.into());
				}
			}
		}

		<Classes<T>>::mutate(fragment_hash, |fragment| {
			if let Some(fragment) = fragment {
				for id in existing..(existing + quantity) {
					let id = id + 1u64;
					let cid = Compact(id);

					<Fragments<T>>::insert(
						(fragment_hash, cid, Compact(1)),
						InstanceData {
							permissions: fragment.permissions,
							created_at: current_block_number,
							custom_data: data,
						},
					);

					<CopiesCount<T>>::insert((fragment_hash, cid), 1);

					<Inventory<T>>::append(to.clone(), fragment_hash, (cid, Compact(1)));

					<Owners<T>>::append(fragment_hash, to.clone(), (cid, Compact(1)));

					Self::deposit_event(Event::InventoryAdded(to.clone(), *fragment_hash, (id, 1)));
				}
				<EditionsCount<T>>::insert(fragment_hash, existing + quantity);
			}
		});

		Ok(())
	}
}

use crate::*;

// pub use pallet_protos::dummy_data::ProtoFragment;
pub use copied_from_pallet_protos::ProtoFragment;

mod copied_from_pallet_protos {

	use super::*;
	

	pub fn compute_data_hash(data: &Vec<u8>) -> Hash256 {
		blake2_256(&data)
	}
	
	// use base58::ToBase58;
	// pub fn compute_data_cid(data: &Vec<u8>) -> Vec<u8> {
	
	// 	let hash = compute_data_hash(data);
	
	// 	let cid = [&CID_PREFIX[..], &hash[..]].concat();
	// 	let cid = cid.to_base58();
	// 	let cid = [&b"z"[..], cid.as_bytes()].concat();
	
	// 	cid
	// }


	#[derive(Clone)]
	pub struct ProtoFragment {
		pub references: Vec<Hash256>,
		pub category: Categories,
		pub tags: Vec<Vec<u8>>,
		pub linked_asset: Option<pallet_protos::LinkedAsset>,
		pub include_cost: Option<u64>,
		pub data: Vec<u8>,
	}
	impl ProtoFragment {
		pub fn get_proto_hash(&self) -> Hash256 {
			compute_data_hash(&self.data)
		}

		// pub fn get_proto_cid(&self) -> Vec<u8> {
		// 	compute_data_cid(&self.data)
		// }
	}
}

use protos::permissions::FragmentPerms;

use sp_clamor::Hash256;

use protos::categories::{Categories, TextCategories};

pub struct Definition { // "Definition" is short for "Fragment Definition"
	pub proto_fragment: ProtoFragment,

	pub metadata: FragmentMetadata<u64>,
	pub permissions: FragmentPerms,

	pub unique: Option<UniqueOptions>,

	pub max_supply: Option<u64>,
}

impl Definition {
	pub fn get_definition_id(&self) -> Hash128 {
		blake2_128(
			&[
				&self.proto_fragment.get_proto_hash()[..],
				&self.metadata.name.encode(),
				&self.metadata.currency.encode(),
			]
			.concat(),
		)
	}

	pub fn get_vault_account_id(&self) -> sp_core::ed25519::Public { 
		let hash = blake2_256(&[&b"fragments-vault"[..], &self.get_definition_id()].concat());
		sp_core::ed25519::Public::from_raw(hash)
	}
}


pub struct Publish {
	pub definition: Definition,

	pub price: u128, // price per instance

	/// * `quantity` (*optional*) - **Maximum amount of Fragment Instances** that **can be bought** 
	pub quantity: Option<u64>,

	pub expires: Option<u64>,

	/// If the Fragment instance represents a **stack of stackable items** (for e.g gold coins or arrows - https://runescape.fandom.com/wiki/Stackable_items), 
	/// the **number of items** to **top up** in the **stack of stackable items**
	pub amount: Option<u64>,
}


pub struct Mint {
	pub definition: Definition,
	pub buy_options: FragmentBuyOptions,
	pub amount: Option<u64>
}


pub struct Buy {
	pub publish: Publish,
	pub buy_options: FragmentBuyOptions,
}

pub struct Give {
	pub mint: Mint,
	pub edition_id: u64,
	pub copy_id: u64,
	pub to: sp_core::ed25519::Public,
	pub new_permissions: Option<FragmentPerms>,
	pub expiration: Option<u64>, 
}

pub struct CreateAccount { // Creates Account for a Fragment Instance
	pub mint: Mint,
	pub edition_id: u64,
	pub copy_id: u64,
}


/// NOTE: All `ProtoFragment`-type fields found in `DummyData` have no references
pub struct DummyData {
	pub definition: Definition,

	pub publish: Publish,
	pub publish_with_max_supply: Publish,

	pub mint_non_unique: Mint,
	pub mint_unique: Mint,

	pub buy_non_unique: Buy,
	pub buy_unique: Buy,

	pub give_no_copy_perms: Give,
	pub give_copy_perms: Give,

	pub create_account: CreateAccount,

	pub account_id: sp_core::ed25519::Public,
	pub account_id_second: sp_core::ed25519::Public,
}

impl DummyData {
	pub fn new() -> Self {

		let definition = Definition {
			proto_fragment: ProtoFragment {
				references: Vec::new(),
				category: Categories::Text(TextCategories::Plain),
				tags: Vec::new(),
				linked_asset: None,
				include_cost: Some(111),
				data: "0x111".as_bytes().to_vec(),
			},
			metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None },
			permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER,
			unique: Some(UniqueOptions { mutable: true }),
			max_supply: Some(111),
		};

		let publish = Publish {
			definition: Definition {
				proto_fragment: ProtoFragment {
					references: Vec::new(),
					category: Categories::Text(TextCategories::Plain),
					tags: Vec::new(),
					linked_asset: None,
					include_cost: Some(222),
					data: "0x222".as_bytes().to_vec(),
				},
				metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
				permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER,
				unique: None, 
				max_supply: None,
			},
			price: 2,
			quantity: Some(222),
			expires: None,
			amount: None, 
		};

		let publish_with_max_supply = Publish {
			definition: Definition {
				proto_fragment: ProtoFragment {
					references: Vec::new(),
					category: Categories::Text(TextCategories::Plain),
					tags: Vec::new(),
					linked_asset: None,
					include_cost: Some(333),
					data: "0x333".as_bytes().to_vec(),
				},
				metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None },
				permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER,
				unique: None, 
				max_supply: Some(1234), // with max supply!
			},
			price: 3,
			quantity: Some(123),
			expires: None,
			amount: None, 
		};


		let mint_non_unique = Mint {
			definition: Definition {
				proto_fragment: ProtoFragment {
					references: Vec::new(),
					category: Categories::Text(TextCategories::Plain),
					tags: Vec::new(),
					linked_asset: None,
					include_cost: Some(444),
					data: "0x444".as_bytes().to_vec(),
				},
				metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
				permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER,
				unique: None, 
				max_supply: Some(1234),
			},
			buy_options: FragmentBuyOptions::Quantity(123),
			amount: None,
		};

		let mint_unique = Mint {
			definition: Definition {
				proto_fragment: ProtoFragment {
					references: Vec::new(),
					category: Categories::Text(TextCategories::Plain),
					tags: Vec::new(),
					linked_asset: None,
					include_cost: Some(555),
					data: "0x555".as_bytes().to_vec(),
				},
				metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
				permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER,
				unique: Some(UniqueOptions { mutable: false }), 
				max_supply: Some(1234),
			},
			buy_options: FragmentBuyOptions::UniqueData(b"I Dati".to_vec()),
			amount: None,
		};


		let buy_non_unique = Buy {
			publish: Publish { 
				definition: Definition { 
					proto_fragment: ProtoFragment {
						references: Vec::new(),
						category: Categories::Text(TextCategories::Plain),
						tags: Vec::new(),
						linked_asset: None,
						include_cost: Some(666),
						data: "0x666".as_bytes().to_vec(),
					},
					metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
					permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER, 
					unique: None, 
					max_supply: Some(1234) 
				}, 
				price: 6, 
				quantity: Some(123), 
				expires: Some(999), 
				amount: None, 
			},
			buy_options: FragmentBuyOptions::Quantity(123),
		};

		let buy_unique = Buy {
			publish: Publish { 
				definition: Definition { 
					proto_fragment: ProtoFragment {
						references: Vec::new(),
						category: Categories::Text(TextCategories::Plain),
						tags: Vec::new(),
						linked_asset: None,
						include_cost: Some(777),
						data: "0x777".as_bytes().to_vec(),
					},
					metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
					permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER, 
					unique: Some(UniqueOptions { mutable: false }), 
					max_supply: Some(1234) 
				}, 
				price: 6, 
				quantity: Some(123), 
				expires: Some(999), 
				amount: None, 
			},
			buy_options: FragmentBuyOptions::UniqueData(b"I Dati".to_vec()),
		};



		let give_no_copy_perms = Give {
			mint: Mint {
				definition: Definition {
					proto_fragment: ProtoFragment {
						references: Vec::new(),
						category: Categories::Text(TextCategories::Plain),
						tags: Vec::new(),
						linked_asset: None,
						include_cost: Some(888),
						data: "0x888".as_bytes().to_vec(),
					},
					metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
					permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER, // no copy perms
					unique: Some(UniqueOptions { mutable: false }), 
					max_supply: Some(1234),
				},
				buy_options: FragmentBuyOptions::UniqueData(b"I Dati".to_vec()),
				amount: None,
			},
			edition_id: 1,
			copy_id: 1,
			to: sp_core::ed25519::Public::from_raw([69u8; 32]),
			new_permissions: Some(FragmentPerms::NONE),
			expiration: Some(999),
		};

		let give_copy_perms = Give {
			mint: Mint {
				definition: Definition {
					proto_fragment: ProtoFragment {
						references: Vec::new(),
						category: Categories::Text(TextCategories::Plain),
						tags: Vec::new(),
						linked_asset: None,
						include_cost: Some(999),
						data: "0x999".as_bytes().to_vec(),
					},
					metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
					permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER | FragmentPerms::COPY, // copy perms
					unique: Some(UniqueOptions { mutable: false }), 
					max_supply: Some(1234),
				},
				buy_options: FragmentBuyOptions::UniqueData(b"I Dati".to_vec()),
				amount: None,
			},
			edition_id: 1,
			copy_id: 1,
			to: sp_core::ed25519::Public::from_raw([69u8; 32]),
			new_permissions: Some(FragmentPerms::NONE),
			expiration: Some(999),
		};

		let create_account = CreateAccount {
			mint: Mint {
				definition: Definition {
					proto_fragment: ProtoFragment {
						references: Vec::new(),
						category: Categories::Text(TextCategories::Plain),
						tags: Vec::new(),
						linked_asset: None,
						include_cost: Some(101010),
						data: "0x101010".as_bytes().to_vec(),
					},
					metadata: FragmentMetadata { name: b"Il Nome".to_vec(), currency: None},
					permissions: FragmentPerms::EDIT | FragmentPerms::TRANSFER,
					unique: Some(UniqueOptions { mutable: false }), 
					max_supply: Some(1234),
				},
				buy_options: FragmentBuyOptions::UniqueData(b"I Dati".to_vec()),
				amount: None,
			},
			edition_id: 1,
			copy_id: 1,		
		};

		Self {
			definition: definition,

			publish: publish,
			publish_with_max_supply: publish_with_max_supply,

			mint_non_unique: mint_non_unique,
			mint_unique: mint_unique,

			buy_non_unique: buy_non_unique,
			buy_unique: buy_unique,

			give_copy_perms: give_copy_perms,
			give_no_copy_perms: give_no_copy_perms,

			create_account: create_account,

			account_id: sp_core::ed25519::Public::from_raw([1u8; 32]),
			account_id_second: sp_core::ed25519::Public::from_raw([2u8; 32]),
		}
	}
}

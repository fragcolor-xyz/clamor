use crate as pallet_fragments;
use crate::mock;

use crate::*;

use crate::dummy_data::*;

use crate::mock::*;

use frame_support::{assert_noop, assert_ok};
use protos::permissions::FragmentPerms;


use copied_from_pallet_protos::upload as upload;
mod copied_from_pallet_protos {
	use super::*;

	pub fn upload(signer: <Test as frame_system::Config>::AccountId, proto: &ProtoFragment) -> DispatchResult {
		ProtosPallet::upload(
			Origin::signed(signer), 
			proto.references.clone(), 
			proto.category.clone(), 
			proto.tags.clone(), 
			proto.linked_asset.clone(), 
			proto.include_cost.map(|cost| Compact::from(cost)), 
			proto.data.clone(),
		)
	}
}



use create_tests::create as create;
mod create_tests {
	use super::*;

	pub fn create(
		signer: <Test as frame_system::Config>::AccountId,
		definition: &Definition,
	) -> DispatchResult {
		FragmentsPallet::create(
			Origin::signed(signer),
			definition.proto_fragment.get_proto_hash(),
			definition.metadata.clone(),
			definition.permissions,
			definition.unique.clone(),
			definition.max_supply,
		)
	}


	#[test]
	fn create_should_work() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let current_block_number = System::block_number(); 

			let definition = dd.definition;

			_ = upload(dd.account_id, &definition.proto_fragment);

			assert_ok!(create(dd.account_id, &definition));

			// TODO - what does `T::Currency::reserve()` do in the `create` extrinsic

			let correct_definition_struct = FragmentDefinition {
				proto_hash: definition.proto_fragment.get_proto_hash(),
				metadata: definition.metadata.clone(),
				permissions: definition.permissions.clone(),
				unique: definition.unique.clone(),
				max_supply: definition.max_supply.map(|max_supply| Compact::from(max_supply)),
				creator: dd.account_id,
				created_at: current_block_number
			};

			assert_eq!(
				<Definitions<Test>>::get(&definition.get_definition_id()).unwrap(), 
				correct_definition_struct
			);

			assert!(
				<Proto2Fragments<Test>>::get(&definition.proto_fragment.get_proto_hash()).unwrap()
				.contains(&definition.get_definition_id())
			);

			let event = System::events().pop().expect("Expected at least one EventRecord to be found").event;
        	assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::DefinitionCreated(definition.get_definition_id())
				)
			);

		});
	}


	#[test]
	fn create_should_not_work_if_fragment_definition_already_exists() {
		
		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let definition = dd.definition;

			_ = upload(dd.account_id, &definition.proto_fragment);

			_ = create(dd.account_id, &definition);

			assert_noop!(
				create(dd.account_id, &definition),
				Error::<Test>::AlreadyExist
			);

		});

	}

	#[test]
	fn create_should_not_work_if_proto_does_not_exist() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let definition = dd.definition;

			assert_noop!(
				create(dd.account_id, &definition),
				Error::<Test>::ProtoNotFound
			);

		});

	}

	#[test]
	fn create_should_not_work_if_user_does_not_own_proto() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let definition = dd.definition;

			_ = upload(dd.account_id, &definition.proto_fragment);

			assert_noop!(
				create(dd.account_id_second, &definition),
				Error::<Test>::NoPermission
			);

		});

	}

	#[test]
	fn create_should_not_work_if_proto_is_detached() {
		todo!()
	}


}

use publish_tests::publish_ as publish_;
mod publish_tests {
	use super::*;

	pub fn publish_(
		signer: <Test as frame_system::Config>::AccountId,
		publish: &Publish,
	) -> DispatchResult {
		FragmentsPallet::publish(
			Origin::signed(signer),
			publish.definition.get_definition_id(),
			publish.price,
			publish.quantity,
			publish.expires,
			publish.amount,
		)
	}

	#[test]
	fn publish_should_work() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			_ = upload(dd.account_id, &publish.definition.proto_fragment);
			_ = create(dd.account_id, &publish.definition);

			assert_ok!(publish_(dd.account_id, &publish));

			let correct_publishing_data_struct = PublishingData {
				price: Compact::from(publish.price),
				units_left: publish.quantity.map(|quantity| Compact::from(quantity)),
				expiration: publish.expires,
				amount: publish.amount.map(|amount| Compact::from(amount)),
			};

			assert_eq!(
				<Publishing<Test>>::get(&publish.definition.get_definition_id()).unwrap(), 
				correct_publishing_data_struct
			);

			let event = System::events().pop().expect("Expected at least one EventRecord to be found").event;
        	assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::Publishing { 
						fragment_hash: publish.definition.get_definition_id()
					}
				)
			);

		});

	}

	#[test]
	fn publish_should_not_work_if_user_does_not_own_the_fragment_definition() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			_ = upload(dd.account_id, &publish.definition.proto_fragment);
			_ = create(dd.account_id, &publish.definition);

			assert_noop!(
				publish_(dd.account_id_second, &publish),
				Error::<Test>::NoPermission
			);

		});
	}

	#[test]
	fn publish_should_not_work_if_fragment_definition_does_not_exist() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			assert_noop!(
				publish_(dd.account_id, &publish),
				Error::<Test>::NotFound
			);

		});
	}

	#[test]
	fn publish_should_not_work_if_fragment_definition_is_currently_published() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			_ = upload(dd.account_id, &publish.definition.proto_fragment);
			_ =create(dd.account_id, &publish.definition);
			_ = publish_(dd.account_id, &publish);

			assert_noop!(
				publish_(dd.account_id, &publish),
				Error::<Test>::SaleAlreadyOpen
			);

		});
	}

	#[test]
	fn publish_should_not_work_if_the_quantity_parameter_is_none_but_the_fragment_definition_has_a_max_supply() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish_with_max_supply = dd.publish_with_max_supply;

			_ = upload(dd.account_id, &publish_with_max_supply.definition.proto_fragment);
			_ = create(dd.account_id, &publish_with_max_supply.definition);

			assert_noop!(
				publish_(
					dd.account_id, 
					&Publish { 
						quantity: None,
						..publish_with_max_supply
					}
				),
				Error::<Test>::ParamsNotValid
			);

		});	
	}

	#[test]
	fn publish_should_work_if_the_quantity_to_publish_is_lesser_than_or_equal_to_the_max_supply_of_the_fragment_definition() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish_with_max_supply = dd.publish_with_max_supply;

			_ = upload(dd.account_id, &publish_with_max_supply.definition.proto_fragment);
			_ = create(dd.account_id, &publish_with_max_supply.definition);

			assert_ok!(
				publish_(
					dd.account_id, 
					&Publish { 
						quantity: publish_with_max_supply.definition.max_supply,
						..publish_with_max_supply
					}
				)
			);

		});		

	}

	#[test]
	fn publish_should_not_work_if_the_quantity_to_publish_is_greater_than_the_max_supply_of_the_fragment_definition() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish_with_max_supply = dd.publish_with_max_supply;

			_ = upload(dd.account_id, &publish_with_max_supply.definition.proto_fragment);
			_ = create(dd.account_id, &publish_with_max_supply.definition);

			assert_noop!(
				publish_(
					dd.account_id, 
					&Publish { 
						quantity: publish_with_max_supply.definition.max_supply.map(|max_supply| max_supply + 1),
						..publish_with_max_supply
					}
				),
				Error::<Test>::MaxSupplyReached
			);

		});		

	}

	#[test]
	fn publish_should_not_work_if_proto_is_detached() {
		todo!()
	}


}

mod unpublish_tests {
	
	use super::*;

	pub fn unpublish_(
		signer: <Test as frame_system::Config>::AccountId,
		definition: &Definition,
	) -> DispatchResult {
		FragmentsPallet::unpublish(
			Origin::signed(signer), 
			definition.get_definition_id()
		)
	}

	#[test]
	fn unpublish_should_work() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			_ = upload(dd.account_id, &publish.definition.proto_fragment);
			_ = create(dd.account_id, &publish.definition);
			_ = publish_(dd.account_id, &publish);

			assert_ok!(unpublish_(dd.account_id, &publish.definition));

			assert_eq!(
				<Publishing<Test>>::contains_key(&publish.definition.get_definition_id()),
				false
			);

			let event = System::events().pop().expect("Expected at least one EventRecord to be found").event;
        	assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::Unpublishing { 
						fragment_hash: publish.definition.get_definition_id()
					}
				)
			);

		});

	}

	#[test]
	fn upublish_should_not_work_if_user_does_not_own_the_fragment_definition() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			_ = upload(dd.account_id, &publish.definition.proto_fragment);
			_ = create(dd.account_id, &publish.definition);
			_ = publish_(dd.account_id, &publish);

			assert_noop!(
				unpublish_(dd.account_id_second, &publish.definition),
				Error::<Test>::NoPermission
			);

		});
		
	}

	#[test]
	fn unpublish_should_not_work_if_fragment_definition_does_not_exist() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			assert_noop!(
				unpublish_(dd.account_id, &publish.definition),
				Error::<Test>::NotFound
			);

		});

	}

	#[test]
	fn unpublish_should_not_work_if_fragment_definition_is_not_published() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let publish = dd.publish;

			_ = upload(dd.account_id, &publish.definition.proto_fragment);
			_ = create(dd.account_id, &publish.definition);

			assert_noop!(
				unpublish_(dd.account_id, &publish.definition),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::NotFound
			);

		});
	}

	#[test]
	fn unpublish_should_not_work_if_proto_is_detached() {
		todo!()
	}

}



use mint_tests::mint_ as mint_;
mod mint_tests {
	use super::*;

	pub fn mint_(
		signer: <Test as frame_system::Config>::AccountId,
		mint: &Mint,
	) -> DispatchResult {
		FragmentsPallet::mint(
			Origin::signed(signer), 
			mint.definition.get_definition_id(), 
			mint.buy_options.clone(),
			mint.amount.clone()
		)
	}


	#[test]
	fn mint_should_work_if_the_options_parameter_is_not_unique() {
		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let current_block_number = System::block_number(); 

			let mint_non_unique = dd.mint_non_unique;

			_ = upload(dd.account_id, &mint_non_unique.definition.proto_fragment);
			_ = create(dd.account_id, &mint_non_unique.definition);

			assert_ok!(mint_(dd.account_id, &mint_non_unique));

			let correct_fragment_instance_struct = FragmentInstance {
				permissions: mint_non_unique.definition.permissions,
				created_at: current_block_number,
				custom_data: None,
				expiring_at: None,
				amount: mint_non_unique.amount.map(|amount| Compact::from(amount)), 
			};

			let quantity = match mint_non_unique.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => panic!(),
			};
			

			for edition_id in 1..=quantity {

				assert_eq!(
					<Fragments<Test>>::get(
						(mint_non_unique.definition.get_definition_id(), edition_id, 1)
					).unwrap(),
					correct_fragment_instance_struct
				);

				assert_eq!(
					<CopiesCount<Test>>::get(
						(mint_non_unique.definition.get_definition_id(), edition_id)
					).unwrap(),
					Compact(1)
				);

				assert!(
					<Inventory<Test>>::get(
						dd.account_id, mint_non_unique.definition.get_definition_id()
					).unwrap()
					.contains(&(Compact(edition_id), Compact(1)))
				);

				assert!(
					<Owners<Test>>::get(
						mint_non_unique.definition.get_definition_id(), dd.account_id
					)
					.unwrap()
					.contains(&(Compact(edition_id), Compact(1)))
				);

				let event = System::events()[2 + (edition_id - 1) as usize].event.clone(); // we do `2 +` because events were also emitted when we did `upload()` and `create()`
				assert_eq!(
					event, 
					mock::Event::from(
						pallet_fragments::Event::InventoryAdded{
							account_id: dd.account_id,
							fragment_hash: mint_non_unique.definition.get_definition_id(),
							fragment_id: (edition_id, 1)
						}
					)
				);


			}

			assert_eq!(
				<EditionsCount<Test>>::get(mint_non_unique.definition.get_definition_id()).unwrap(), 
				Compact(quantity)
			);


		});
	}

	// TODO see if the data was indexed
	#[test]
	fn mint_should_work_if_the_options_parameter_is_unique() {
		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let current_block_number = System::block_number(); 

			let mint_unique = dd.mint_unique;

			_ = upload(dd.account_id, &mint_unique.definition.proto_fragment);
			_ = create(dd.account_id, &mint_unique.definition);

			assert_ok!(mint_(dd.account_id, &mint_unique));

			let correct_fragment_instance_struct = FragmentInstance {
				permissions: mint_unique.definition.permissions,
				created_at: current_block_number,
				custom_data: match mint_unique.buy_options {
					FragmentBuyOptions::UniqueData(data) => Some(blake2_256(&data)),
					_ => panic!(),
				},
				expiring_at: None,
				amount: mint_unique.amount.map(|amount| Compact::from(amount)), 
			};

			

			assert_eq!(
				<Fragments<Test>>::get(
					(mint_unique.definition.get_definition_id(), 1, 1)
				).unwrap(),
				correct_fragment_instance_struct
			);

			assert_eq!(
				<CopiesCount<Test>>::get(
					(mint_unique.definition.get_definition_id(), 1)
				).unwrap(),
				Compact(1)
			);

			assert!(
				<Inventory<Test>>::get(
					dd.account_id, mint_unique.definition.get_definition_id()
				).unwrap()
				.contains(&(Compact(1), Compact(1)))
			);

			assert!(
				<Owners<Test>>::get(
					mint_unique.definition.get_definition_id(), dd.account_id
				)
				.unwrap()
				.contains(&(Compact(1), Compact(1)))
			);

			let event = System::events()[2 as usize].event.clone(); // we write `2` because events were also emitted when we did `upload()` and `create()`
			assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::InventoryAdded {
						account_id: dd.account_id,
						fragment_hash: mint_unique.definition.get_definition_id(),
						fragment_id: (1, 1)
					}
				)
			);

			assert_eq!(
				<EditionsCount<Test>>::get(mint_unique.definition.get_definition_id()).unwrap(), 
				Compact(1)
			);


		});
	}

	#[test]
	fn mint_should_not_work_if_the_user_is_not_the_owner_of_the_fragment_definition() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint = dd.mint_non_unique;

			_ = upload(dd.account_id, &mint.definition.proto_fragment);
			_ = create(dd.account_id, &mint.definition);

			assert_noop!(
				mint_(dd.account_id_second, &mint),
				Error::<Test>::NoPermission
			);

		});
		
	}

	#[test]
	fn mint_should_not_work_if_fragment_definition_does_not_exist() {
		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint = dd.mint_non_unique;

			_ = upload(dd.account_id, &mint.definition.proto_fragment);

			assert_noop!(
				mint_(dd.account_id, &mint),
				Error::<Test>::NotFound
			);

		});
	}

	#[test]
	fn mint_should_work_if_the_quantity_to_create_is_lesser_than_or_equal_to_the_max_supply_of_the_fragment_definition() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint = dd.mint_non_unique;

			assert!(mint.definition.unique.is_none()); // we use non-unique because we can create multiple instances in a single extrinsic call (basically we're lazy)
			assert!(mint.definition.max_supply.is_some()); // max supply exists

			_ = upload(dd.account_id, &mint.definition.proto_fragment);
			_ = create(dd.account_id, &mint.definition);

			assert_ok!(
				mint_(
					dd.account_id, 
					&Mint {
						buy_options: FragmentBuyOptions::Quantity(mint.definition.max_supply.unwrap()),
						..mint
					}
				)
			);		


		});
		
	}

	#[test] 
	fn mint_should_not_work_if_the_quantity_to_create_is_greater_than_the_max_supply_of_the_fragment_definition() {
		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint = dd.mint_non_unique;

			assert!(mint.definition.unique.is_none()); // we use non-unique because we can create multiple instances in a single extrinsic call (basically we're lazy)
			assert!(mint.definition.max_supply.is_some()); // max supply exists

			_ = upload(dd.account_id, &mint.definition.proto_fragment);
			_ = create(dd.account_id, &mint.definition);

			assert_noop!(
				mint_(
					dd.account_id, 
					&Mint {
						buy_options: FragmentBuyOptions::Quantity(mint.definition.max_supply.unwrap() + 1),
						..mint
					}
				),
				Error::<Test>::MaxSupplyReached
			);	


		});

	}

	#[test]
	fn mint_should_not_work_if_the_options_parameter_is_unique_but_the_the_fragment_definition_is_not_unique() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint_non_unique = dd.mint_non_unique; // fragment definition is not unique

			_ = upload(dd.account_id, &mint_non_unique.definition.proto_fragment);
			_ = create(dd.account_id, &mint_non_unique.definition);

			assert_noop!(
				mint_(
					dd.account_id, 
					&Mint {
						buy_options: FragmentBuyOptions::UniqueData(b"I dati".to_vec()), // options parameter is unique
						..mint_non_unique
					}
				),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::ParamsNotValid
			);


		});
		

	}

	#[test]
	fn mint_should_not_work_if_the_options_parameter_is_not_unique_but_the_the_fragment_definition_is_unique() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint_unique = dd.mint_unique; // fragment definition is unique

			_ = upload(dd.account_id, &mint_unique.definition.proto_fragment);
			_ = create(dd.account_id, &mint_unique.definition);

			assert_noop!(
				mint_(
					dd.account_id, 
					&Mint {
						buy_options: FragmentBuyOptions::Quantity(123), // options parameter is not unique
						..mint_unique
					}
				),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::ParamsNotValid
			);

		});

	}


	#[test]
	fn mint_should_not_work_if_fragment_instance_was_already_created_with_the_same_unique_data() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mint_unique = dd.mint_unique; 

			_ = upload(dd.account_id, &mint_unique.definition.proto_fragment);
			_ = create(dd.account_id, &mint_unique.definition);

			_ = mint_(dd.account_id, &mint_unique);

			assert_noop!(
				mint_(dd.account_id, &mint_unique),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::UniqueDataExists
			);

		});			

	}

	#[test]
	fn mint_should_not_work_if_proto_is_detached() {
		todo!()
	}

	
}


mod buy_tests {
	use super::*;

	fn buy_(
		signer: <Test as frame_system::Config>::AccountId,
		buy: &Buy,
	) -> DispatchResult {
		FragmentsPallet::buy(
			Origin::signed(signer), 
			buy.publish.definition.get_definition_id(), 
			buy.buy_options.clone(),
		)
	}


	#[test]
	fn buy_should_work_if_options_parameter_is_not_unique() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let current_block_number = System::block_number();

			let buy_non_unique = dd.buy_non_unique;

			_ = upload(dd.account_id, &buy_non_unique.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy_non_unique.publish.definition);
			
			_ = publish_(dd.account_id, &buy_non_unique.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy_non_unique.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy_non_unique.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			assert_ok!(buy_(dd.account_id_second, &buy_non_unique));

			let correct_fragment_instance_struct = FragmentInstance {
				permissions: buy_non_unique.publish.definition.permissions,
				created_at: current_block_number,
				custom_data: None,
				expiring_at: None, // newly created Fragment Instance doesn't have an expiration date - confirm with @sinkingsugar 
				amount: buy_non_unique.publish.amount.map(|amount| Compact::from(amount)), 
			};
			
			for edition_id in 1..=quantity {

				assert_eq!(
					<Fragments<Test>>::get(
						(buy_non_unique.publish.definition.get_definition_id(), edition_id, 1)
					).unwrap(),
					correct_fragment_instance_struct
				);

				assert_eq!(
					<CopiesCount<Test>>::get(
						(buy_non_unique.publish.definition.get_definition_id(), edition_id)
					).unwrap(),
					Compact(1)
				);

				assert!(
					<Inventory<Test>>::get(
						dd.account_id_second, buy_non_unique.publish.definition.get_definition_id()
					).unwrap()
					.contains(&(Compact(edition_id), Compact(1)))
				);

				assert!(
					<Owners<Test>>::get(
						buy_non_unique.publish.definition.get_definition_id(), dd.account_id_second
					)
					.unwrap()
					.contains(&(Compact(edition_id), Compact(1)))
				);

				assert_eq!(
					System::events()[5 + edition_id as usize].event.clone(), // we do `5 +` because events were also emitted when we did `upload()` and `create()` and `publish()` and `deposite_creating()` (note: `deposit_creating()` emits 3 events) 
					mock::Event::from(
						pallet_fragments::Event::InventoryAdded {
							account_id: dd.account_id_second,
							fragment_hash: buy_non_unique.publish.definition.get_definition_id(),
							fragment_id: (edition_id, 1)
						}
					)
				);


			}

			assert_eq!(
				<EditionsCount<Test>>::get(buy_non_unique.publish.definition.get_definition_id()).unwrap(), 
				Compact(quantity)
			);

			assert_eq!(
				System::events()[System::events().len() - 3].event, 
				mock::Event::from(
					frame_system::Event::NewAccount {
						account: buy_non_unique.publish.definition.get_vault_account_id(),
					}
				)
			);
			assert_eq!(
				System::events()[System::events().len() - 2].event,
				mock::Event::from(
					pallet_balances::Event::Endowed {
						account: buy_non_unique.publish.definition.get_vault_account_id(),
						free_balance: buy_non_unique.publish.price.saturating_mul(quantity as u128),
					}
				)
			);
			assert_eq!(
				System::events()[System::events().len() - 1].event, 
				mock::Event::from(
					pallet_balances::Event::Transfer {
						from: dd.account_id_second,
						to: buy_non_unique.publish.definition.get_vault_account_id(),
						amount: buy_non_unique.publish.price.saturating_mul(quantity as u128),
					}
				)
			);

		});

	}


	// TODO see if the data was indexed
	#[test]
	fn buy_should_work_if_the_options_parameter_is_unique() {
		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let current_block_number = System::block_number(); 

			let buy_unique = dd.buy_unique;

			_ = upload(dd.account_id, &buy_unique.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy_unique.publish.definition);

			_ = publish_(dd.account_id, &buy_unique.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy_unique.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy_unique.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			assert_ok!(buy_(dd.account_id_second, &buy_unique));

			let correct_fragment_instance_struct = FragmentInstance {
				permissions: buy_unique.publish.definition.permissions,
				created_at: current_block_number,
				custom_data: match buy_unique.buy_options {
					FragmentBuyOptions::UniqueData(data) => Some(blake2_256(&data)),
					_ => panic!(),
				},
				expiring_at: None,
				amount: buy_unique.publish.amount.map(|amount| Compact::from(amount)), 
			};

			assert_eq!(
				<Fragments<Test>>::get(
					(buy_unique.publish.definition.get_definition_id(), 1, 1)
				).unwrap(),
				correct_fragment_instance_struct
			);

			assert_eq!(
				<CopiesCount<Test>>::get(
					(buy_unique.publish.definition.get_definition_id(), 1)
				).unwrap(),
				Compact(1)
			);

			assert!(
				<Inventory<Test>>::get(
					dd.account_id_second, buy_unique.publish.definition.get_definition_id()
				).unwrap()
				.contains(&(Compact(1), Compact(1)))
			);

			assert!(
				<Owners<Test>>::get(
					buy_unique.publish.definition.get_definition_id(), dd.account_id_second
				)
				.unwrap()
				.contains(&(Compact(1), Compact(1)))
			);

			
			assert_eq!(
				System::events()[6 as usize].event.clone(), // we do `6` because events were also emitted when we did `upload()` and `create()` and `publish()` and `deposite_creating()` (note: `deposit_creating()` emits 3 events)
				mock::Event::from(
					pallet_fragments::Event::InventoryAdded {
						account_id: dd.account_id_second,
						fragment_hash: buy_unique.publish.definition.get_definition_id(),
						fragment_id: (1, 1)
					}
				)
			);

			assert_eq!(
				<EditionsCount<Test>>::get(buy_unique.publish.definition.get_definition_id()).unwrap(), 
				Compact(1)
			);


			assert_eq!(
				System::events()[System::events().len() - 3].event, 
				mock::Event::from(
					frame_system::Event::NewAccount {
						account: buy_unique.publish.definition.get_vault_account_id(),
					}
				)
			);
			assert_eq!(
				System::events()[System::events().len() - 2].event,
				mock::Event::from(
					pallet_balances::Event::Endowed {
						account: buy_unique.publish.definition.get_vault_account_id(),
						free_balance: buy_unique.publish.price.saturating_mul(quantity as u128),
					}
				)
			);
			assert_eq!(
				System::events()[System::events().len() - 1].event, 
				mock::Event::from(
					pallet_balances::Event::Transfer {
						from: dd.account_id_second,
						to: buy_unique.publish.definition.get_vault_account_id(),
						amount: buy_unique.publish.price.saturating_mul(quantity as u128),
					}
				)
			);


		});
	}

	#[test]
	fn buy_should_not_work_if_the_fragment_definition_is_not_published() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let buy = dd.buy_non_unique;

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);

			assert_noop!(
				buy_(dd.account_id_second, &buy),
				Error::<Test>::NotFound,
			);
			

		});

	}

	#[test]
	fn buy_should_work_if_user_has_sufficient_balance_in_pallet_balances() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let buy = dd.buy_non_unique;

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			assert_ok!(buy_(dd.account_id_second, &buy));



		});

	}

	#[test]
	fn buy_should_not_work_if_user_has_insufficient_balance_in_pallet_balances() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let buy = dd.buy_non_unique;

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy.publish.price.saturating_mul(quantity as u128) + minimum_balance - 1
			);

			assert_noop!(
				buy_(dd.account_id_second, &buy),
				Error::<Test>::InsufficientBalance
			);



		});
	}


	#[test]
	fn buy_should_work_if_user_has_sufficient_balance_in_pallet_assets() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut buy = dd.buy_non_unique;

			buy.publish.definition.metadata.currency = Some(69);

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			todo!();

			assert_ok!(buy_(dd.account_id_second, &buy));



		});

	}

	#[test]
	fn buy_should_not_work_if_user_has_insufficient_balance_in_pallet_assets() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut buy = dd.buy_non_unique;

			buy.publish.definition.metadata.currency = Some(69);

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			todo!();

			assert_noop!(
				buy_(dd.account_id_second, &buy),
				Error::<Test>::InsufficientBalance
			);



		});
	}

	#[test]
	fn buy_should_work_if_the_quantity_to_create_is_lesser_than_or_equal_to_the_published_quantity_of_the_fragment_definition() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mut buy = dd.buy_non_unique;
			buy.buy_options = FragmentBuyOptions::Quantity(buy.publish.quantity.unwrap()); // equal to the max supply of the fragment definition

			assert!(buy.publish.definition.unique.is_none()); // we use non-unique because we can create multiple instances in a single extrinsic call (basically we're lazy)
			assert!(buy.publish.definition.max_supply.is_some()); // max supply exists

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			assert_ok!(
				buy_(
					dd.account_id_second, 
					&buy
				)
			);		


		});
	}

	#[test]
	fn buy_should_not_work_if_the_quantity_to_create_is_greater_than_the_published_quantity_of_the_fragment_definition() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let mut buy = dd.buy_non_unique;
			buy.buy_options = FragmentBuyOptions::Quantity(buy.publish.quantity.unwrap() + 1); // greater than the max supply of the fragment definition

			assert!(buy.publish.definition.unique.is_none()); // we use non-unique because we can create multiple instances in a single extrinsic call (basically we're lazy)
			assert!(buy.publish.definition.max_supply.is_some()); // max supply exists

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			assert_noop!(
				buy_(
					dd.account_id_second, 
					&buy
				),
				Error::<Test>::PublishedQuantityReached
			);	
			
		});

	}

	#[test]
	fn buy_should_not_work_if_the_options_parameter_is_unique_but_the_the_fragment_definition_is_not_unique() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let buy_non_unique = dd.buy_non_unique; // fragment definition is not unique

			_ = upload(dd.account_id, &buy_non_unique.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy_non_unique.publish.definition);
			_ = publish_(dd.account_id, &buy_non_unique.publish);

			// We deposit (Price * 1) + `minimum_balance` 
			// because that's all we need to deposit if our options parameter is unique
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy_non_unique.publish.price.saturating_mul(1 as u128) + minimum_balance
			);

			assert_noop!(
				buy_(
					dd.account_id_second, 
					&Buy {
						buy_options: FragmentBuyOptions::UniqueData(b"I dati".to_vec()), // options parameter is unique
						..buy_non_unique
					}
				),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::ParamsNotValid
			);

		});
	}

	#[test]
	fn buy_should_not_work_if_the_options_parameter_is_not_unique_but_the_the_fragment_definition_is_unique() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let buy_unique = dd.buy_unique; // fragment definition is unique

			_ = upload(dd.account_id, &buy_unique.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy_unique.publish.definition);
			_ = publish_(dd.account_id, &buy_unique.publish);

			// Since our options parameter is `FragmentBuyOptions::Quantity(123)`, 
			// we deposit (Price * 123) + `minimum_balance`
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy_unique.publish.price.saturating_mul(123 as u128) + minimum_balance
			);

			assert_noop!(
				buy_(
					dd.account_id_second, 
					&Buy {
						buy_options: FragmentBuyOptions::Quantity(123), // options parameter is not unique
						..buy_unique
					}
				),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::ParamsNotValid
			);

		});
	}

	#[test]
	fn buy_should_work_if_the_sale_has_not_expired() {
		new_test_ext().execute_with(|| {
			
			let dd = DummyData::new();

			let buy = dd.buy_non_unique;

			assert!(buy.publish.expires.is_some()); // sale must have an expiration

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			run_to_block(buy.publish.expires.unwrap() - 1);

			assert_ok!(buy_(dd.account_id_second, &buy));


		});
		
	}

	#[test]
	fn buy_should_not_work_if_the_sale_has_expired() {
		new_test_ext().execute_with(|| {
			
			let dd = DummyData::new();

			let buy = dd.buy_non_unique;

			assert!(buy.publish.expires.is_some()); // sale must have an expiration

			_ = upload(dd.account_id, &buy.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy.publish.definition);
			_ = publish_(dd.account_id, &buy.publish);

			// Deposit `quantity` to buyer's account
			let quantity = match buy.buy_options {
				FragmentBuyOptions::Quantity(amount) => u64::from(amount),
				_ => 1u64,
			};
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy.publish.price.saturating_mul(quantity as u128) + minimum_balance
			);

			run_to_block(buy.publish.expires.unwrap());

			assert_noop!(
				buy_(dd.account_id_second, &buy),
				Error::<Test>::Expired,
			);


		});

	}

	#[test]
	fn buy_should_not_work_if_fragment_instance_was_already_created_with_the_same_unique_data() {

		new_test_ext().execute_with(|| {

			let dd = DummyData::new();

			let buy_unique = dd.buy_unique; 

			_ = upload(dd.account_id, &buy_unique.publish.definition.proto_fragment);
			_ = create(dd.account_id, &buy_unique.publish.definition);
			_ = publish_(dd.account_id, &buy_unique.publish);

			// We deposit (Price * 1) + `minimum_balance` 
			// because that's all we need to deposit if our options parameter is unique
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy_unique.publish.price.saturating_mul(1 as u128) + minimum_balance
			);

			_ = buy_(dd.account_id_second, &buy_unique);

			// We deposit (Price * 1) + `minimum_balance` 
			// because that's all we need to deposit if our options parameter is unique
			let minimum_balance = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::minimum_balance();
			_ = <BalancesPallet as Currency<<Test as frame_system::Config>::AccountId>>::deposit_creating(
				&dd.account_id_second, 
				buy_unique.publish.price.saturating_mul(1 as u128) + minimum_balance
			);

			assert_noop!(
				buy_(dd.account_id_second, &buy_unique),
				// this error does not exist yet @sinkingsugar ???
				Error::<Test>::UniqueDataExists
			);

		});		
		
	}

	#[test]
	fn buy_should_not_work_if_proto_is_detached() {
		todo!()
	}


}

mod give_tests {
	use super::*;

	pub fn give_(
		// Review whether these many parameters are appropriate/needed @karan
		signer: <Test as frame_system::Config>::AccountId,
		give: &Give
	) -> DispatchResult {
		FragmentsPallet::give(
			Origin::signed(signer),
			give.mint.definition.get_definition_id(),
			give.edition_id,
			give.copy_id,
			give.to,
			give.new_permissions,
			give.expiration,
		)
	}


	#[test]
	fn give_should_work_if_the_fragment_instance_does_not_have_copy_permissions() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let give = dd.give_no_copy_perms;

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			assert_ok!(give_(dd.account_id, &give));

			assert_eq!(
				<Owners<Test>>::get(
					give.mint.definition.get_definition_id(), dd.account_id
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id))
				),
				false
			);
			assert_eq!(
				<Inventory<Test>>::get(
					dd.account_id, give.mint.definition.get_definition_id()
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id))
				),
				false
			);

			assert_eq!(
				<Owners<Test>>::get(
					give.mint.definition.get_definition_id(), give.to
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id))
				),
				true
			);
			assert_eq!(
				<Inventory<Test>>::get(
					give.to, give.mint.definition.get_definition_id()
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id))
				),
				true
			);

			assert_eq!(
				<Fragments<Test>>::get(
					(give.mint.definition.get_definition_id(), give.edition_id, give.copy_id)
				).unwrap()
				.permissions,
				give.new_permissions.unwrap()
			);

			let event = System::events().get(System::events().len() - 2).expect("Expected at least two EventRecords to be found").event.clone();
        	assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::InventoryRemoved {
						account_id: dd.account_id,
						fragment_hash: give.mint.definition.get_definition_id(), 
						fragment_id: (give.edition_id, give.copy_id)
					}
				)
			);
			
			let event = System::events().pop().expect("Expected at least one EventRecord to be found").event;
        	assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::InventoryAdded {
						account_id: give.to,
						fragment_hash: give.mint.definition.get_definition_id(), 
						fragment_id: (give.edition_id, give.copy_id)
					}
				)
			);



		});
	}


	#[test]
	fn give_should_work_if_the_fragment_instance_has_copy_permissions() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let give = dd.give_copy_perms;

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			assert_ok!(give_(dd.account_id, &give));

			assert_eq!(
				<Owners<Test>>::get(
					give.mint.definition.get_definition_id(), dd.account_id
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id))
				),
				true
			);
			assert_eq!(
				<Inventory<Test>>::get(
					dd.account_id, give.mint.definition.get_definition_id()
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id))
				),
				true
			);

			assert_eq!(
				<Owners<Test>>::get(
					give.mint.definition.get_definition_id(), give.to
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id + 1))
				),
				true
			);
			assert_eq!(
				<Inventory<Test>>::get(
					give.to, give.mint.definition.get_definition_id()
				).unwrap().contains(
					&(Compact(give.edition_id), Compact(give.copy_id + 1))
				),
				true
			);

			assert_eq!(
				<CopiesCount<Test>>::get(
					(give.mint.definition.get_definition_id(), give.edition_id)
				).unwrap(),
				Compact(2)
			);

			assert_eq!(
				<Fragments<Test>>::get(
					(give.mint.definition.get_definition_id(), give.edition_id, give.copy_id + 1)
				).unwrap()
				.permissions,
				give.new_permissions.unwrap()
			);

			assert!(
				<Expirations<Test>>::get(&give.expiration.unwrap()).unwrap().contains(
					&(give.mint.definition.get_definition_id(), Compact(give.edition_id), Compact(give.copy_id + 1))
				)
			);


			let event = System::events().pop().expect("Expected at least one EventRecord to be found").event;
        	assert_eq!(
				event, 
				mock::Event::from(
					pallet_fragments::Event::InventoryAdded {
						account_id: give.to,
						fragment_hash: give.mint.definition.get_definition_id(), 
						fragment_id: (give.edition_id, give.copy_id + 1)
					}
				)
			);



		});

	}


	#[test]
	fn give_should_not_work_if_the_user_does_not_own_the_fragment_instance() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let give = dd.give_no_copy_perms;

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			assert_noop!(
				give_(dd.account_id_second, &give), 
				Error::<Test>::NotFound // should this be the error that is thrown @sinkingsugar. It doesn't sound appropriate
			);
		});
		
	}


	#[test]
	fn give_should_work_if_the_new_permissions_are_more_or_equally_restrictive() {

		use itertools::Itertools;

		let all_perms = vec![FragmentPerms::EDIT, FragmentPerms::TRANSFER, FragmentPerms::COPY]; 
		assert_eq!(all_perms.clone().into_iter().fold(FragmentPerms::NONE, |acc, x| acc | x), FragmentPerms::ALL);
		let all_perms_except_transfer = vec![FragmentPerms::EDIT, FragmentPerms::COPY]; 
		assert_eq!(
			all_perms_except_transfer.clone().into_iter().fold(FragmentPerms::TRANSFER, |acc, x| acc | x), 
			FragmentPerms::ALL
		);

		for num_combos in 0..=all_perms_except_transfer.len() {
			for perms in all_perms_except_transfer.clone().into_iter().combinations(num_combos) {

				let mut perms = perms.clone();
				perms.push(FragmentPerms::TRANSFER); // TRANSFER must be included, since we want to give it

				for num_combos in 0..=perms.len() {
					for new_permissions_parameter in perms.clone().into_iter().combinations(num_combos) {

						new_test_ext().execute_with(|| {
							let dd = DummyData::new();
			
							let mut give = dd.give_no_copy_perms;
		
							give.mint.definition.permissions = perms.clone().into_iter().fold(
								FragmentPerms::NONE, 
								|acc, x| acc | x
							);
							
							_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
							_ = create(dd.account_id, &give.mint.definition);
							_ = mint_(dd.account_id, &give.mint);

							give.new_permissions = Some(
								new_permissions_parameter.into_iter().fold(
									FragmentPerms::NONE, 
									|acc, x| acc | x
								)
							);

							// println!(
							// 	"current permissions are: {:?}, new_permissions are: {:?}", 
							// 	give.mint.definition.permissions,
							// 	give.new_permissions.clone()
							// );

							assert_ok!(give_(dd.account_id, &give));
		
						});
					}
				}


			}

		}
	}

	#[test]
	fn give_should_not_work_if_the_new_permissions_are_less_restrictive() {

		use itertools::Itertools;
		
		let all_perms = vec![FragmentPerms::EDIT, FragmentPerms::TRANSFER, FragmentPerms::COPY]; 
		assert_eq!(all_perms.clone().into_iter().fold(FragmentPerms::NONE, |acc, x| acc | x), FragmentPerms::ALL);
		let all_perms_except_transfer = vec![FragmentPerms::EDIT, FragmentPerms::COPY]; 
		assert_eq!(
			all_perms_except_transfer.clone().into_iter().fold(FragmentPerms::TRANSFER, |acc, x| acc | x), 
			FragmentPerms::ALL
		);

		for num_combos in 0..all_perms_except_transfer.len() {
			for perms in all_perms_except_transfer.clone().into_iter().combinations(num_combos) {

				let mut perms = perms.clone();
				perms.push(FragmentPerms::TRANSFER); // TRANSFER must be included, since we want to give it 

				let mut possible_additional_perms = all_perms.clone();
				possible_additional_perms.retain(|x| !perms.contains(x));

				for num_combos in 1..=possible_additional_perms.len() {
					for new_perms in possible_additional_perms.clone().into_iter().combinations(num_combos)  {

						let mut new_permissions_parameter = new_perms;
						new_permissions_parameter.extend(perms.clone());

						new_test_ext().execute_with(|| {
							let dd = DummyData::new();
			
							let mut give = dd.give_no_copy_perms;
		
							give.mint.definition.permissions = perms.clone().into_iter().fold(
								FragmentPerms::NONE, 
								|acc, x| acc | x
							);
				
							_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
							_ = create(dd.account_id, &give.mint.definition);
							_ = mint_(dd.account_id, &give.mint);

							give.new_permissions = Some(
								new_permissions_parameter.into_iter().fold(
									FragmentPerms::NONE, 
									|acc, x| acc | x
								)
							);
	
							assert_noop!(
								give_(dd.account_id, &give),
								Error::<Test>::NoPermission
							);
						});
					}
				}

			}

		}
	}
		

	#[test]
	fn give_should_not_work_if_the_fragment_instance_does_not_have_the_transfer_permission() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut give = dd.give_no_copy_perms;

			give.mint.definition.permissions = FragmentPerms::ALL - FragmentPerms::TRANSFER; // does not have transfer permission

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			assert_noop!(
				give_(dd.account_id, &give),
				Error::<Test>::NoPermission
			);
		});
	}

	#[test]
	fn give_should_work_if_the_fragment_instance_expires_after_the_current_block_number() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut give = dd.give_copy_perms;

			assert!(give.expiration.is_some()); 
			give.new_permissions =  Some(FragmentPerms::TRANSFER | FragmentPerms::COPY); // ensure that the duplicated instance can also be used to create duplicates

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			_ = give_(dd.account_id, &give); 

			run_to_block(give.expiration.unwrap() - 1); 

			let give_second_time = Give {
				mint: give.mint,
				edition_id: give.edition_id,
				copy_id: give.copy_id + 1,
				to: dd.account_id_second,
				new_permissions: None,
				expiration: None, 
			};

			assert_ok!(give_(give.to, &give_second_time));


		});

	}

	#[test]
	fn give_should_not_work_if_the_fragment_instance_expires_before_or_at_the_current_block_number() {

		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut give = dd.give_copy_perms;

			assert!(give.expiration.is_some()); 
			give.new_permissions =  Some(FragmentPerms::TRANSFER | FragmentPerms::COPY); // ensure that the duplicated instance can also be used to create duplicates

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			_ = give_(dd.account_id, &give); 

			run_to_block(give.expiration.unwrap()); 

			println!("current block number is: {}", System::block_number());

			let give_second_time = Give {
				mint: give.mint,
				edition_id: give.edition_id,
				copy_id: give.copy_id + 1,
				to: dd.account_id_second,
				new_permissions: None,
				expiration: None, 
			};

			assert_noop!(
				give_(give.to, &give_second_time), 
				Error::<Test>::NotFound
			);


		});

	}


	#[test]
	fn give_should_work_if_the_expiration_parameter_is_greater_than_the_current_block_number() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let current_block_number = System::block_number();

			let give = Give {
				expiration: Some(current_block_number + 1),
				..dd.give_no_copy_perms
			};

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			assert_ok!(give_(dd.account_id, &give));
		});
	}

	#[test]
	fn give_should_not_work_if_the_expiration_parameter_is_lesser_than_or_equal_to_the_current_block_number() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let current_block_number = System::block_number();

			let give = Give {
				expiration: Some(current_block_number),
				..dd.give_no_copy_perms
			};

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			assert_noop!(
				give_(dd.account_id, &give),
				Error::<Test>::ParamsNotValid
			);
		});
	}

	// TODO - test to check if duplicated Instance's expirations changes
	#[test] 
	fn give_should_change_the_expiration_of_the_duplicated_fragment_instance_if_the_expiration_parameter_is_lesser_than_it() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut give = dd.give_copy_perms;

			assert!(give.expiration.is_some()); 
			give.new_permissions =  Some(FragmentPerms::TRANSFER | FragmentPerms::COPY); // ensure that the duplicated instance can also be used to create duplicates

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			_ = give_(dd.account_id, &give); 

			let give_second_time = Give {
				mint: give.mint,
				edition_id: give.edition_id,
				copy_id: give.copy_id + 1,
				to: dd.account_id_second,
				new_permissions: None,
				expiration: Some(give.expiration.unwrap() - 1), // expiration parameter is greater than it 
			};

			assert_ok!(give_(give.to, &give_second_time));

			assert!(
				!<Expirations<Test>>::get(give.expiration.unwrap()).unwrap().contains(
					&(
						give_second_time.mint.definition.get_definition_id(), 
						Compact(give_second_time.edition_id), 
						Compact(give_second_time.copy_id + 1)
					)
				)					
			);

			assert!(
				<Expirations<Test>>::get(give.expiration.unwrap() - 1).unwrap().contains(
					&(
						give_second_time.mint.definition.get_definition_id(), 
						Compact(give_second_time.edition_id), 
						Compact(give_second_time.copy_id + 1)
					)
				)					
			);


		});

	}

	#[test]
	fn give_should_not_change_the_expiration_of_the_duplicated_fragment_instance_if_the_expiration_parameter_is_greater_than_it() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let mut give = dd.give_copy_perms;

			assert!(give.expiration.is_some()); 
			give.new_permissions =  Some(FragmentPerms::TRANSFER | FragmentPerms::COPY); // ensure that the duplicated instance can also be used to create duplicates

			_ = upload(dd.account_id, &give.mint.definition.proto_fragment);
			_ = create(dd.account_id, &give.mint.definition);
			_ = mint_(dd.account_id, &give.mint);

			_ = give_(dd.account_id, &give); 

			let give_second_time = Give {
				mint: give.mint,
				edition_id: give.edition_id,
				copy_id: give.copy_id + 1,
				to: dd.account_id_second,
				new_permissions: None,
				expiration: Some(give.expiration.unwrap() + 1), // expiration parameter is greater than it 
			};

			assert_ok!(give_(give.to, &give_second_time));

			assert!(
				!<Expirations<Test>>::contains_key(give.expiration.unwrap() + 1)		
			);

			assert!(
				<Expirations<Test>>::get(give.expiration.unwrap()).unwrap().contains(
					&(
						give_second_time.mint.definition.get_definition_id(), 
						Compact(give_second_time.edition_id), 
						Compact(give_second_time.copy_id + 1)
					)
				)					
			);


		});
	}


}


mod create_account_tests {
	use super::*;

	pub fn create_account_(
		signer: <Test as frame_system::Config>::AccountId,
		create_account: &CreateAccount
	) -> DispatchResult {
		FragmentsPallet::create_account(
			Origin::signed(signer),
			create_account.mint.definition.get_definition_id(),
			create_account.edition_id,
			create_account.copy_id,
		)
	}

	#[test]
	fn create_account_should_work() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let create_account = dd.create_account;

			_ = upload(dd.account_id, &create_account.mint.definition.proto_fragment);
			_ = create(dd.account_id, &create_account.mint.definition);
			_ = mint_(dd.account_id, &create_account.mint);

			assert_ok!(create_account_(dd.account_id, &create_account));

			todo!();



		});
	}

	#[test]
	fn create_account_should_not_work_if_the_user_is_not_the_owner_of_the_fragment_instance() {
		new_test_ext().execute_with(|| {
			let dd = DummyData::new();

			let create_account = dd.create_account;

			_ = upload(dd.account_id, &create_account.mint.definition.proto_fragment);
			_ = create(dd.account_id, &create_account.mint.definition);
			_ = mint_(dd.account_id, &create_account.mint);

			assert_noop!(
				create_account_(dd.account_id_second, &create_account),
				Error::<Test>::NotFound // should this be the error that is thrown @sinkingsugar. It doesn't sound appropriate
			);

		});
	}


}

pub use crate as pallet_fragments;
use crate::*;
use frame_support::{
	parameter_types,
	traits::{ConstU32, ConstU64, ConstU128},
};
use frame_system;
use sp_core::{ed25519::Signature, H256};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		ProtosPallet: pallet_protos::{Pallet, Call, Storage, Event<T>},
		FragmentsPallet: pallet_fragments::{Pallet, Call, Storage, Event<T>},
		DetachPallet: pallet_detach::{Pallet, Call, Storage, Event<T>},
		BalancesPallet: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		AssetsPallet: pallet_assets::{Pallet, Call, Storage, Event<T>},
		AccountsPallet: pallet_accounts::{Pallet, Call, Storage, Event<T>},
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
	pub StorageBytesMultiplier: u64 = 10;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::ed25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<2>;
}

pub type Extrinsic = TestXt<Call, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

impl pallet_randomness_collective_flip::Config for Test {}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

impl pallet_assets::Config for Test {
	type Event = Event;
	type Balance = u128;
	type AssetId = u64;
	type Currency = ();
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type AssetDeposit = ConstU32<1>;
	type AssetAccountDeposit = ConstU32<10>;
	type MetadataDepositBase = ConstU32<1>;
	type MetadataDepositPerByte = ConstU32<1>;
	type ApprovalDeposit = ConstU32<1>;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type WeightInfo = ();
	type Extra = ();
}

impl pallet_protos::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type StorageBytesMultiplier = StorageBytesMultiplier;
	type StakeLockupPeriod = ConstU64<100800>; // one week
}

impl pallet_accounts::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type EthChainId = ConstU64<5>; // goerli
	type EthFragContract = ();
	type EthConfirmations = ConstU64<1>;
	type Threshold = ConstU64<1>;
	type AuthorityId = pallet_accounts::crypto::FragAuthId;
}

impl pallet_proxy::Config for Test {
	type Event = Event;
	type Call = Call;
	type Currency = ();
	type ProxyType = ();
	type ProxyDepositBase = ConstU32<1>;
	type ProxyDepositFactor = ConstU32<1>;
	type MaxProxies = ConstU32<4>;
	type WeightInfo = ();
	type MaxPending = ConstU32<2>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = ConstU32<1>;
	type AnnouncementDepositFactor = ConstU32<1>;
}

impl pallet_fragments::Config for Test {
	type Event = Event;
	type WeightInfo = ();
}

impl pallet_detach::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type AuthorityId = pallet_detach::crypto::DetachAuthId;
}

impl pallet_timestamp::Config for Test {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(t);

	ext.execute_with(|| System::set_block_number(1)); // if we don't execute this line, Events are not emitted from extrinsics (I don't know why this is the case though)

	ext
}


/// Simulate block production
/// 
/// A simple way of doing this is by incrementing the System module's block number between `on_initialize` and `on_finalize` calls 
/// from all modules with `System::block_number()` as the sole input. 
/// While it is important for runtime code to cache calls to storage or the system module, the test environment scaffolding should 
/// prioritize readability to facilitate future maintenance.
/// 
/// Source: https://docs.substrate.io/v3/runtime/testing/
pub fn run_to_block(n: u64) {
    while System::block_number() < n {

		use frame_support::traits::{OnInitialize, OnFinalize}; 

        if System::block_number() > 0 {
            FragmentsPallet::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        // FragmentsPallet::on_initialize(System::block_number()); // Commented out since this function (`on_finalize`) doesn't exist in pallets/fragments/src/lib.rs
    }
}
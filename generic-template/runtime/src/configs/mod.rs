pub mod asset_config;
pub use asset_config::AssetType;
pub mod governance;
pub mod weight;
pub mod xcm_config;

use asset_config::*;
#[cfg(feature = "tanssi")]
use cumulus_pallet_parachain_system::ExpectParentIncluded;
#[cfg(feature = "async-backing")]
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
#[cfg(not(feature = "async-backing"))]
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
#[cfg(not(feature = "tanssi"))]
use frame_support::PalletId;
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU64, Contains, EitherOf,
        EitherOfDiverse, Everything, Nothing, TransformOrigin,
    },
    weights::{ConstantMultiplier, Weight},
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureRootWithSuccess, EnsureSigned,
};
pub use governance::origins::pallet_custom_origins;
use governance::{origins::Treasurer, tracks, Spender, WhitelistedCaller};
use openzeppelin_pallet_abstractions::{
    impl_openzeppelin_assets, impl_openzeppelin_governance, impl_openzeppelin_system,
    impl_openzeppelin_xcm, AssetsConfig, AssetsWeight, GovernanceConfig, GovernanceWeight,
    SystemConfig, SystemWeight, XcmConfig, XcmWeight,
};
#[cfg(not(feature = "tanssi"))]
use openzeppelin_pallet_abstractions::{
    impl_openzeppelin_consensus, ConsensusConfig, ConsensusWeight,
};
#[cfg(feature = "tanssi")]
use openzeppelin_pallet_abstractions::{impl_openzeppelin_tanssi, TanssiConfig, TanssiWeight};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
#[cfg(not(feature = "tanssi"))]
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, IdentityLookup},
    Perbill,
};
use xcm::latest::{prelude::*, InteriorLocation};
#[cfg(not(feature = "runtime-benchmarks"))]
use xcm_builder::ProcessXcmMessage;
use xcm_builder::{
    AllowExplicitUnpaidExecutionFrom, AllowTopLevelPaidExecutionFrom,
    DenyReserveTransferToRelayChain, DenyThenTry, EnsureXcmOrigin, FixedWeightBounds,
    FrameTransactionalProcessor, TakeWeightCredit, TrailingSetTopicAsId, WithComputedOrigin,
    WithUniqueTopic,
};
use xcm_config::*;
use xcm_executor::XcmExecutor;
use xcm_primitives::AsAssetType;

#[cfg(feature = "runtime-benchmarks")]
use crate::benchmark::{OpenHrmpChannel, PayWithEnsure};
#[cfg(not(feature = "tanssi"))]
use crate::{
    constants::HOURS,
    types::{BlockNumber, CollatorSelectionUpdateOrigin, ConsensusHook},
    SessionKeys,
};
use crate::{
    constants::{
        currency::{deposit, CENTS, EXISTENTIAL_DEPOSIT, MICROCENTS},
        AVERAGE_ON_INITIALIZE_RATIO, DAYS, MAXIMUM_BLOCK_WEIGHT, MAX_BLOCK_LENGTH,
        NORMAL_DISPATCH_RATIO, SLOT_DURATION,
    },
    types::{
        AccountId, AssetId, AssetKind, Balance, Beneficiary, Block, Hash,
        MessageQueueServiceWeight, Nonce, PriceForSiblingParachainDelivery, ProxyType,
        TreasuryAccount, TreasuryInteriorLocation, TreasuryPalletId, TreasuryPaymaster, Version,
    },
    weights::{BlockExecutionWeight, ExtrinsicBaseWeight},
    AllPalletsWithSystem, AssetManager, Balances, MessageQueue, Oracle, OracleMembership,
    OriginCaller, PalletInfo, ParachainInfo, ParachainSystem, PolkadotXcm, Preimage, Referenda,
    Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin,
    RuntimeTask, Scheduler, System, Timestamp, Treasury, WeightToFee, XcmpQueue,
};
#[cfg(not(feature = "tanssi"))]
use crate::{Aura, CollatorSelection, Session};

// OpenZeppelin runtime wrappers configuration
pub struct OpenZeppelinRuntime;
impl SystemConfig for OpenZeppelinRuntime {
    type AccountId = AccountId;
    #[cfg(feature = "tanssi")]
    type ConsensusHook = ExpectParentIncluded;
    #[cfg(not(feature = "tanssi"))]
    type ConsensusHook = ConsensusHook;
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type Lookup = AccountIdLookup<AccountId, ()>;
    #[cfg(feature = "tanssi")]
    type OnTimestampSet = ();
    #[cfg(not(feature = "tanssi"))]
    type OnTimestampSet = Aura;
    type PreimageOrigin = EnsureRoot<AccountId>;
    type ProxyType = ProxyType;
    type SS58Prefix = ConstU16<42>;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
    type Version = Version;
}
#[cfg(not(feature = "tanssi"))]
impl ConsensusConfig for OpenZeppelinRuntime {
    type CollatorSelectionUpdateOrigin = CollatorSelectionUpdateOrigin;
}
impl GovernanceConfig for OpenZeppelinRuntime {
    type ConvictionVoteLockingPeriod = ConstU32<{ 7 * DAYS }>;
    type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<AccountId>, WhitelistedCaller>;
    type ReferendaAlarmInterval = ConstU32<1>;
    type ReferendaCancelOrigin = EnsureRoot<AccountId>;
    type ReferendaKillOrigin = EnsureRoot<AccountId>;
    type ReferendaSlash = Treasury;
    type ReferendaSubmissionDeposit = ConstU128<{ 3 * CENTS }>;
    type ReferendaSubmitOrigin = EnsureSigned<AccountId>;
    type ReferendaUndecidingTimeout = ConstU32<{ 14 * DAYS }>;
    type TreasuryInteriorLocation = TreasuryInteriorLocation;
    type TreasuryPalletId = TreasuryPalletId;
    type TreasuryPayoutSpendPeriod = ConstU32<{ 30 * DAYS }>;
    type TreasuryRejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
    type TreasurySpendOrigin = TreasurySpender;
    type TreasurySpendPeriod = ConstU32<{ 6 * DAYS }>;
    type WhitelistOrigin = EnsureRoot<AccountId>;
}
impl XcmConfig for OpenZeppelinRuntime {
    type AccountIdToLocation = AccountIdToLocation;
    type AddSupportedAssetOrigin = EnsureRoot<AccountId>;
    type AssetFeesFilter = AssetFeesFilter;
    type AssetTransactors = AssetTransactors;
    type BaseXcmWeight = BaseXcmWeight;
    type CurrencyId = CurrencyId;
    type CurrencyIdToLocation = CurrencyIdToLocation<AsAssetType<AssetId, AssetType, AssetManager>>;
    type DerivativeAddressRegistrationOrigin = EnsureRoot<AccountId>;
    type EditSupportedAssetOrigin = EnsureRoot<AccountId>;
    type FeeManager = FeeManager;
    type HrmpManipulatorOrigin = EnsureRoot<AccountId>;
    type HrmpOpenOrigin = EnsureRoot<AccountId>;
    type LocalOriginToLocation = LocalOriginToLocation;
    type LocationToAccountId = LocationToAccountId;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    type MaxHrmpRelayFee = MaxHrmpRelayFee;
    type MessageQueueHeapSize = ConstU32<{ 64 * 1024 }>;
    type MessageQueueMaxStale = ConstU32<8>;
    type MessageQueueServiceWeight = MessageQueueServiceWeight;
    type ParachainMinFee = ParachainMinFee;
    type PauseSupportedAssetOrigin = EnsureRoot<AccountId>;
    type RelayLocation = RelayLocation;
    type RemoveSupportedAssetOrigin = EnsureRoot<AccountId>;
    type Reserves = Reserves;
    type ResumeSupportedAssetOrigin = EnsureRoot<AccountId>;
    type SelfLocation = SelfLocation;
    type SelfReserve = SelfReserve;
    type SovereignAccountDispatcherOrigin = EnsureRoot<AccountId>;
    type Trader = pallet_xcm_weight_trader::Trader<Runtime>;
    type TransactorReserveProvider =
        xcm_primitives::AbsoluteAndRelativeReserve<SelfLocationAbsolute>;
    type Transactors = Transactors;
    type UniversalLocation = UniversalLocation;
    type WeightToFee = WeightToFee;
    type XcmAdminOrigin = EnsureRoot<AccountId>;
    type XcmFeesAccount = TreasuryAccount;
    type XcmOriginToTransactDispatchOrigin = XcmOriginToTransactDispatchOrigin;
    type XcmSender = XcmRouter;
    type XcmWeigher = XcmWeigher;
    type XcmpQueueControllerOrigin = EnsureRoot<AccountId>;
    type XcmpQueueMaxInboundSuspended = ConstU32<1000>;
    type XtokensReserveProviders = ReserveProviders;
}

parameter_types! {
    pub RootOperatorAccountId: AccountId = AccountId::from([0xffu8; 32]);
}

impl AssetsConfig for OpenZeppelinRuntime {
    type AccountId = AccountId;
    type ApprovalDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AssetAccountDeposit = ConstU128<{ deposit(1, 16) }>;
    type AssetDeposit = ConstU128<{ 10 * CENTS }>;
    type AssetId = AssetId;
    type AssetRegistrar = AssetRegistrar;
    type AssetRegistrarMetadata = AssetRegistrarMetadata;
    type AssetType = AssetType;
    type AssetsToBlockAuthor = parachains_common::impls::AssetsToBlockAuthor<Runtime, ()>;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type ForeignAssetModifierOrigin = EnsureRoot<AccountId>;
    type FungiblesToAccount = TreasuryAccount;
    type RootOperatorAccountId = RootOperatorAccountId;
    type Timestamp = Timestamp;
    type WeightToFee = WeightToFee;
}

#[cfg(feature = "tanssi")]
impl TanssiConfig for OpenZeppelinRuntime {
    type AuthorInherent = pallet_author_inherent::weights::SubstrateWeight<Runtime>;
    type AuthoritiesNothing = pallet_cc_authorities_noting::weights::SubstrateWeight<Runtime>;
}
impl_openzeppelin_system!(OpenZeppelinRuntime);
#[cfg(not(feature = "tanssi"))]
impl_openzeppelin_consensus!(OpenZeppelinRuntime);
impl_openzeppelin_governance!(OpenZeppelinRuntime);
impl_openzeppelin_xcm!(OpenZeppelinRuntime);
impl_openzeppelin_assets!(OpenZeppelinRuntime);

#[cfg(feature = "tanssi")]
impl_openzeppelin_tanssi!(OpenZeppelinRuntime);

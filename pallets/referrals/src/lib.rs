// Copyright (C) 2020-2023  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Referrals pallet
//!
//! Support for referrals, referral codes and rewards distribution.
//!
//! ## Overview
//!
//! Referrals give an opportunity to users to earn some rewards from trading activity if the trader
//! used their referral code to link their account to the referrer account.
//!
//! The trader can get back part of the trade fee too if configured.
//!
//! Pallet also provides support for volume-based tiering. Referrer can reached higher Level based on the total amount generated by users of the referrer code.
//! The higher level, the better reward.
//!
//! Rewards are accumulated in the pallet's account and if it is not RewardAsset, it is converted to RewardAsset prior to claim.
//!
//! ### Terminology
//!
//! * **Referral code:**  a string of certain size that identifies the referrer. Must be alphanumeric and upper case.
//! * **Referrer:**  user that registered a code
//! * **Trader:**  user that does a trade
//! * **Reward Asset:**  id of an asset which rewards are paid in. Usually native asset.
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
pub mod migration;
#[cfg(test)]
mod tests;
pub mod traits;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::{DispatchResult, Get};
use frame_support::traits::fungibles::Transfer;
use frame_support::{defensive, ensure, transactional, RuntimeDebug};
use frame_system::{ensure_signed, pallet_prelude::OriginFor};
use hydradx_traits::price::PriceProvider;
use orml_traits::GetByKey;
use scale_info::TypeInfo;
use sp_core::bounded::BoundedVec;
use sp_core::U256;
use sp_runtime::helpers_128bit::multiply_by_rational_with_rounding;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::Rounding;
use sp_runtime::{
	traits::{CheckedAdd, Zero},
	ArithmeticError, DispatchError, Permill,
};

#[cfg(feature = "runtime-benchmarks")]
pub use crate::traits::BenchmarkHelper;

pub use pallet::*;

use weights::WeightInfo;

pub type Balance = u128;
pub type ReferralCode<S> = BoundedVec<u8, S>;

/// Referrer level.
/// Indicates current level of the referrer to determine which reward percentages are used.
#[derive(Hash, Clone, Copy, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum Level {
	None,
	#[default]
	Tier0,
	Tier1,
	Tier2,
	Tier3,
	Tier4,
}

impl Level {
	pub fn next_level(&self) -> Self {
		match self {
			Self::Tier0 => Self::Tier1,
			Self::Tier1 => Self::Tier2,
			Self::Tier2 => Self::Tier3,
			Self::Tier3 => Self::Tier4,
			Self::Tier4 => Self::Tier4,
			Self::None => Self::None,
		}
	}

	pub fn is_max_level(&self) -> bool {
		*self == Self::Tier4
	}

	pub fn increase<T: Config>(self, amount: Balance) -> Self {
		if self.is_max_level() {
			self
		} else {
			let next_level = self.next_level();
			let required = T::LevelVolumeAndRewardPercentages::get(&next_level).0;
			if amount >= required {
				return next_level.increase::<T>(amount);
			}
			self
		}
	}
}

#[derive(Clone, Copy, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FeeDistribution {
	/// Percentage of the fee that goes to the referrer.
	pub referrer: Permill,
	/// Percentage of the fee that goes back to the trader.
	pub trader: Permill,
	/// Percentage of the fee that goes to specific account given by `ExternalAccount` config parameter as reward.r
	pub external: Permill,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode, TypeInfo)]
pub struct AssetAmount<AssetId> {
	asset_id: AssetId,
	amount: Balance,
}

impl<AssetId> AssetAmount<AssetId> {
	pub fn new(asset_id: AssetId, amount: Balance) -> Self {
		Self { asset_id, amount }
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::traits::Convert;
	use frame_support::pallet_prelude::*;
	use frame_support::sp_runtime::ArithmeticError;
	use frame_support::traits::fungibles::{Inspect, Transfer};
	use frame_support::PalletId;
	use hydra_dx_math::ema::EmaPrice;
	use sp_runtime::traits::Zero;

	#[pallet::pallet]
	#[pallet::generate_store(pub(crate) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin that can set asset reward percentages.
		type AuthorityOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Asset type
		type AssetId: frame_support::traits::tokens::AssetId + MaybeSerializeDeserialize;

		/// Support for transfers.
		type Currency: Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Balance>;

		/// Support for asset conversion.
		type Convert: Convert<Self::AccountId, Self::AssetId, Balance, Error = DispatchError>;

		/// Price provider to use for shares calculation.
		type PriceProvider: PriceProvider<Self::AssetId, Price = EmaPrice>;

		/// ID of an asset that is used to distribute rewards in.
		#[pallet::constant]
		type RewardAsset: Get<Self::AssetId>;

		/// Pallet id. Determines account which holds accumulated rewards in various assets.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Registration fee details.
		/// (ID of an asset which fee is to be paid in, Amount, Beneficiary account)
		#[pallet::constant]
		type RegistrationFee: Get<(Self::AssetId, Balance, Self::AccountId)>;

		/// Maximum referral code length.
		#[pallet::constant]
		type CodeLength: Get<u32>;

		// Minimum referral code length.
		#[pallet::constant]
		type MinCodeLength: Get<u32>;

		/// Volume and Global reward percentages for all assets if not specified explicitly for the asset.
		type LevelVolumeAndRewardPercentages: GetByKey<Level, (Balance, FeeDistribution)>;

		/// External account that receives some percentage of the fee. Usually something like staking.
		type ExternalAccount: Get<Option<Self::AccountId>>;

		/// Seed amount that was sent to the reward pot.
		#[pallet::constant]
		type SeedNativeAmount: Get<u128>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: BenchmarkHelper<Self::AssetId, Balance>;
	}

	/// Referral codes
	/// Maps an account to a referral code.
	#[pallet::storage]
	#[pallet::getter(fn referral_account)]
	pub(super) type ReferralCodes<T: Config> =
		StorageMap<_, Blake2_128Concat, ReferralCode<T::CodeLength>, T::AccountId>;

	/// Referral accounts
	#[pallet::storage]
	#[pallet::getter(fn referral_code)]
	pub(super) type ReferralAccounts<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ReferralCode<T::CodeLength>>;

	/// Linked accounts.
	/// Maps an account to a referral account.
	#[pallet::storage]
	#[pallet::getter(fn linked_referral_account)]
	pub(super) type LinkedAccounts<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId>;

	/// Shares of a referral account
	#[pallet::storage]
	#[pallet::getter(fn referrer_shares)]
	pub(super) type ReferrerShares<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Balance, ValueQuery>;

	/// Shares of a trader account
	#[pallet::storage]
	#[pallet::getter(fn trader_shares)]
	pub(super) type TraderShares<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Balance, ValueQuery>;

	/// Total share issuance.
	#[pallet::storage]
	#[pallet::getter(fn total_shares)]
	pub(super) type TotalShares<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// Referer level and total accumulated rewards over time.
	/// Maps referrer account to (Level, Balance). Level indicates current rewards and Balance is used to unlock next level.
	/// Dev note: we use OptionQuery here because this helps to easily determine that an account if referrer account.
	#[pallet::storage]
	#[pallet::getter(fn referrer_level)]
	pub(super) type Referrer<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (Level, Balance), OptionQuery>;

	/// Asset fee distribution rewards information.
	/// Maps (asset_id, level) to asset reward percentages.
	#[pallet::storage]
	#[pallet::getter(fn asset_rewards)]
	pub(super) type AssetRewards<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, T::AssetId, Blake2_128Concat, Level, FeeDistribution, OptionQuery>;

	/// Information about assets that are currently in the rewards pot.
	/// Used to easily determine list of assets that need to be converted.
	#[pallet::storage]
	#[pallet::getter(fn pending_conversions)]
	pub(super) type PendingConversions<T: Config> = CountedStorageMap<_, Blake2_128Concat, T::AssetId, ()>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Referral code has been registered.
		CodeRegistered {
			code: ReferralCode<T::CodeLength>,
			account: T::AccountId,
		},
		/// Referral code has been linked to an account.
		CodeLinked {
			account: T::AccountId,
			code: ReferralCode<T::CodeLength>,
			referral_account: T::AccountId,
		},
		/// Asset has been converted to RewardAsset.
		Converted {
			from: AssetAmount<T::AssetId>,
			to: AssetAmount<T::AssetId>,
		},
		/// Rewards claimed.
		Claimed {
			who: T::AccountId,
			referrer_rewards: Balance,
			trade_rewards: Balance,
		},
		/// New asset rewards has been set.
		AssetRewardsUpdated {
			asset_id: T::AssetId,
			level: Level,
			rewards: FeeDistribution,
		},
		/// Referrer reached new level.
		LevelUp { who: T::AccountId, level: Level },
	}

	#[pallet::error]
	#[cfg_attr(test, derive(PartialEq, Eq))]
	pub enum Error<T> {
		/// Referral code is too long.
		TooLong,
		/// Referral code is too short.
		TooShort,
		/// Referral code contains invalid character. Only alphanumeric characters are allowed.
		InvalidCharacter,
		/// Referral code already exists.
		AlreadyExists,
		/// Provided referral code is invalid. Either does not exist or is too long.
		InvalidCode,
		/// Account is already linked to another referral account.
		AlreadyLinked,
		/// Nothing in the referral pot account for the asset.
		ZeroAmount,
		/// Linking an account to the same referral account is not allowed.
		LinkNotAllowed,
		/// Calculated rewards are more than the fee amount. This can happen if percentages are incorrectly set.
		IncorrectRewardCalculation,
		/// Given referrer and trader percentages exceeds 100% percent.
		IncorrectRewardPercentage,
		/// The account has already a code registered.
		AlreadyRegistered,
		/// Price for given asset pair not found.
		PriceNotFound,
		/// Minimum trading amount for conversion has not been reached.
		ConversionMinTradingAmountNotReached,
		/// Zero amount received from conversion.
		ConversionZeroAmountReceived,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register new referral code.
		///
		/// `origin` pays the registration fee.
		/// `code` is assigned to the given `account`.
		///
		/// Length of the `code` must be at least `T::MinCodeLength`.
		/// Maximum length is limited to `T::CodeLength`.
		/// `code` must contain only alfa-numeric characters and all characters will be converted to upper case.
		///
		/// Parameters:
		/// - `code`: Code to register. Must follow the restrictions.
		///
		/// Emits `CodeRegistered` event when successful.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_code())]
		pub fn register_code(origin: OriginFor<T>, code: ReferralCode<T::CodeLength>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				ReferralAccounts::<T>::get(&who).is_none(),
				Error::<T>::AlreadyRegistered
			);

			ensure!(code.len() >= T::MinCodeLength::get() as usize, Error::<T>::TooShort);

			ensure!(
				code.clone()
					.into_inner()
					.iter()
					.all(|c| char::is_alphanumeric(*c as char)),
				Error::<T>::InvalidCharacter
			);

			let code = Self::normalize_code(code);

			ReferralCodes::<T>::mutate(code.clone(), |v| -> DispatchResult {
				ensure!(v.is_none(), Error::<T>::AlreadyExists);

				let (fee_asset, fee_amount, beneficiary) = T::RegistrationFee::get();
				T::Currency::transfer(fee_asset, &who, &beneficiary, fee_amount, true)?;

				*v = Some(who.clone());
				Referrer::<T>::insert(&who, (Level::default(), Balance::zero()));
				ReferralAccounts::<T>::insert(&who, code.clone());
				Self::deposit_event(Event::CodeRegistered { code, account: who });
				Ok(())
			})
		}

		/// Link a code to an account.
		///
		/// `Code` must be valid registered code. Otherwise `InvalidCode` is returned.
		///
		/// Signer account is linked to the referral account of the code.
		///
		/// Parameters:
		/// - `code`: Code to use to link the signer account to.
		///
		/// Emits `CodeLinked` event when successful.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::link_code())]
		pub fn link_code(origin: OriginFor<T>, code: ReferralCode<T::CodeLength>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let code = Self::normalize_code(code);
			let ref_account = Self::referral_account(&code).ok_or(Error::<T>::InvalidCode)?;

			LinkedAccounts::<T>::mutate(who.clone(), |v| -> DispatchResult {
				ensure!(v.is_none(), Error::<T>::AlreadyLinked);

				ensure!(who != ref_account, Error::<T>::LinkNotAllowed);

				*v = Some(ref_account.clone());
				Self::deposit_event(Event::CodeLinked {
					account: who,
					code,
					referral_account: ref_account,
				});
				Ok(())
			})?;
			Ok(())
		}

		/// Convert accrued asset amount to reward currency.
		///
		/// Parameters:
		/// - `asset_id`: Id of an asset to convert to RewardAsset.
		///
		/// Emits `Converted` event when successful.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::convert())]
		pub fn convert(origin: OriginFor<T>, asset_id: T::AssetId) -> DispatchResult {
			ensure_signed(origin)?;

			let asset_balance = T::Currency::balance(asset_id, &Self::pot_account_id());
			ensure!(asset_balance > 0, Error::<T>::ZeroAmount);

			let total_reward_asset =
				T::Convert::convert(Self::pot_account_id(), asset_id, T::RewardAsset::get(), asset_balance)?;

			PendingConversions::<T>::remove(asset_id);

			Self::deposit_event(Event::Converted {
				from: AssetAmount::new(asset_id, asset_balance),
				to: AssetAmount::new(T::RewardAsset::get(), total_reward_asset),
			});

			Ok(())
		}

		/// Claim accumulated rewards
		///
		/// IF there is any asset in the reward pot, all is converted to RewardCurrency first.
		///
		/// Reward amount is calculated based on the shares of the signer account.
		///
		/// if the signer account is referrer account, total accumulated rewards is updated as well as referrer level if reached.
		///
		/// Emits `Claimed` event when successful.
		#[pallet::call_index(3)]
		#[pallet::weight( {
			let c = PendingConversions::<T>::count() as u64;
			let convert_weight = (<T as Config>::WeightInfo::convert()).saturating_mul(c);
			let w  = <T as Config>::WeightInfo::claim_rewards();
			let one_read = T::DbWeight::get().reads(1_u64);
			w.saturating_add(convert_weight).saturating_add(one_read)
		})]
		pub fn claim_rewards(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			for (asset_id, _) in PendingConversions::<T>::iter() {
				let asset_balance = T::Currency::balance(asset_id, &Self::pot_account_id());
				let r = T::Convert::convert(Self::pot_account_id(), asset_id, T::RewardAsset::get(), asset_balance);
				if let Err(error) = r {
					if error == Error::<T>::ConversionMinTradingAmountNotReached.into()
						|| error == Error::<T>::ConversionZeroAmountReceived.into()
					{
						// We allow these errors to continue claiming as the current amount of asset that needed to be converted
						// has very low impact on the rewards.
					} else {
						return Err(error);
					}
				} else {
					PendingConversions::<T>::remove(asset_id);
				}
			}
			let referrer_shares = ReferrerShares::<T>::take(&who);
			let trader_shares = TraderShares::<T>::take(&who);
			let total_shares = referrer_shares.saturating_add(trader_shares);
			if total_shares == Balance::zero() {
				return Ok(());
			}

			let reward_reserve = T::Currency::balance(T::RewardAsset::get(), &Self::pot_account_id());
			let reward_reserve = reward_reserve.saturating_sub(T::SeedNativeAmount::get());
			let share_issuance = TotalShares::<T>::get();

			let convert_shares = |to_convert: Balance| -> Option<Balance> {
				let shares_hp = U256::from(to_convert);
				let reward_reserve_hp = U256::from(reward_reserve);
				let share_issuance_hp = U256::from(share_issuance);
				let r = shares_hp
					.checked_mul(reward_reserve_hp)?
					.checked_div(share_issuance_hp)?;
				Balance::try_from(r).ok()
			};

			let referrer_rewards = convert_shares(referrer_shares).ok_or(ArithmeticError::Overflow)?;
			let trader_rewards = convert_shares(trader_shares).ok_or(ArithmeticError::Overflow)?;
			let total_rewards = referrer_rewards
				.checked_add(trader_rewards)
				.ok_or(ArithmeticError::Overflow)?;
			ensure!(total_rewards <= reward_reserve, Error::<T>::IncorrectRewardCalculation);

			// Make sure that we can transfer all the rewards if all shares withdrawn.
			let keep_pot_alive = total_shares != share_issuance;

			T::Currency::transfer(
				T::RewardAsset::get(),
				&Self::pot_account_id(),
				&who,
				total_rewards,
				keep_pot_alive,
			)?;
			TotalShares::<T>::mutate(|v| {
				*v = v.saturating_sub(total_shares);
			});
			Referrer::<T>::mutate(who.clone(), |v| {
				if let Some((level, total)) = v {
					*total = total.saturating_add(referrer_rewards);
					let new_level = level.increase::<T>(*total);
					if *level != new_level {
						*level = new_level;
						Self::deposit_event(Event::LevelUp {
							who: who.clone(),
							level: new_level,
						});
					}
				}
			});

			Self::deposit_event(Event::Claimed {
				who,
				referrer_rewards,
				trade_rewards: trader_rewards,
			});
			Ok(())
		}

		/// Set asset reward percentages
		///
		/// Parameters:
		/// - `asset_id`: asset id
		/// - `level`: level
		/// - `rewards`: reward fee percentages
		///
		/// Emits `AssetRewardsUpdated` event when successful.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::set_reward_percentage())]
		pub fn set_reward_percentage(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			level: Level,
			rewards: FeeDistribution,
		) -> DispatchResult {
			T::AuthorityOrigin::ensure_origin(origin)?;

			//ensure that total percentage does not exceed 100%
			ensure!(
				rewards
					.referrer
					.checked_add(&rewards.trader)
					.ok_or(Error::<T>::IncorrectRewardPercentage)?
					.checked_add(&rewards.external)
					.is_some(),
				Error::<T>::IncorrectRewardPercentage
			);

			AssetRewards::<T>::mutate(asset_id, level, |v| {
				*v = Some(rewards);
			});
			Self::deposit_event(Event::AssetRewardsUpdated {
				asset_id,
				level,
				rewards,
			});
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_idle(_n: T::BlockNumber, remaining_weight: Weight) -> Weight {
			let convert_weight = T::WeightInfo::convert();
			if convert_weight.is_zero() {
				return Weight::zero();
			}
			let one_read = T::DbWeight::get().reads(1u64);
			let max_converts = remaining_weight.saturating_sub(one_read).ref_time() / convert_weight.ref_time();

			for asset_id in PendingConversions::<T>::iter_keys().take(max_converts as usize) {
				let asset_balance = T::Currency::balance(asset_id, &Self::pot_account_id());
				let r = T::Convert::convert(Self::pot_account_id(), asset_id, T::RewardAsset::get(), asset_balance);
				if r.is_ok() {
					PendingConversions::<T>::remove(asset_id);
				}
			}
			convert_weight.saturating_mul(max_converts).saturating_add(one_read)
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn pot_account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	pub(crate) fn normalize_code(code: ReferralCode<T::CodeLength>) -> ReferralCode<T::CodeLength> {
		let r = code.into_inner().iter().map(|v| v.to_ascii_uppercase()).collect();
		ReferralCode::<T::CodeLength>::truncate_from(r)
	}

	/// Process trader fee
	/// `source`: account to take the fee from
	/// `trader`: account that does the trade
	///
	/// Returns unused amount on success.
	#[transactional]
	pub fn process_trade_fee(
		source: T::AccountId,
		trader: T::AccountId,
		asset_id: T::AssetId,
		amount: Balance,
	) -> Result<Balance, DispatchError> {
		let Some(price) = T::PriceProvider::get_price(T::RewardAsset::get(), asset_id) else {
			// no price, no fun.
			return Ok(Balance::zero());
		};

		let (level, ref_account) = if let Some(acc) = Self::linked_referral_account(&trader) {
			if let Some((level, _)) = Self::referrer_level(&acc) {
				// Should not really happen, the ref entry should be always there.
				(level, Some(acc))
			} else {
				defensive!("Referrer details not found");
				return Ok(Balance::zero());
			}
		} else {
			(Level::None, None)
		};

		// What is asset fee for this level? if not explicitly set, use global parameter.
		let rewards =
			Self::asset_rewards(asset_id, level).unwrap_or_else(|| T::LevelVolumeAndRewardPercentages::get(&level).1);

		// Rewards
		let external_account = T::ExternalAccount::get();
		let referrer_reward = if ref_account.is_some() {
			rewards.referrer.mul_floor(amount)
		} else {
			0
		};
		let trader_reward = rewards.trader.mul_floor(amount);
		let external_reward = if external_account.is_some() {
			rewards.external.mul_floor(amount)
		} else {
			0
		};
		let total_taken = referrer_reward
			.saturating_add(trader_reward)
			.saturating_add(external_reward);
		ensure!(total_taken <= amount, Error::<T>::IncorrectRewardCalculation);
		T::Currency::transfer(asset_id, &source, &Self::pot_account_id(), total_taken, true)?;

		let referrer_shares = if ref_account.is_some() {
			multiply_by_rational_with_rounding(referrer_reward, price.n, price.d, Rounding::Down)
				.ok_or(ArithmeticError::Overflow)?
		} else {
			0
		};
		let trader_shares = multiply_by_rational_with_rounding(trader_reward, price.n, price.d, Rounding::Down)
			.ok_or(ArithmeticError::Overflow)?;
		let external_shares = if external_account.is_some() {
			multiply_by_rational_with_rounding(external_reward, price.n, price.d, Rounding::Down)
				.ok_or(ArithmeticError::Overflow)?
		} else {
			0
		};

		TotalShares::<T>::mutate(|v| {
			*v = v.saturating_add(
				referrer_shares
					.saturating_add(trader_shares)
					.saturating_add(external_shares),
			);
		});
		if let Some(acc) = ref_account {
			ReferrerShares::<T>::mutate(acc, |v| {
				*v = v.saturating_add(referrer_shares);
			});
		}
		TraderShares::<T>::mutate(trader, |v| {
			*v = v.saturating_add(trader_shares);
		});
		if let Some(acc) = external_account {
			TraderShares::<T>::mutate(acc, |v| {
				*v = v.saturating_add(external_shares);
			});
		}
		if asset_id != T::RewardAsset::get() {
			PendingConversions::<T>::insert(asset_id, ());
		}
		Ok(total_taken)
	}
}

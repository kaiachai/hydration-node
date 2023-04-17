// This file is part of HydraDX.

// Copyright (C) 2020-2022  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use cumulus_pallet_xcmp_queue::XcmDeferFilter;
use frame_support::dispatch::Weight;
use frame_support::traits::{Contains, EnsureOrigin};
use frame_support::{ensure, pallet_prelude::DispatchResult, traits::Get};
use frame_system::ensure_signed_or_root;
use frame_system::pallet_prelude::OriginFor;
use scale_info::TypeInfo;
use sp_core::MaxEncodedLen;
use sp_runtime::traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero};
use sp_runtime::{ArithmeticError, DispatchError, RuntimeDebug};
use xcm::VersionedXcm;

pub mod weights;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

#[cfg(test)]
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::HasCompact;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::Contains;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Identifier for the class of asset.
		type AssetId: Member
			+ Parameter
			+ Default
			+ Copy
			+ HasCompact
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo
			+ AtLeast32BitUnsigned;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	/// TODO:
	#[pallet::getter(fn remove_liquidity_limit_per_asset)]
	pub type LiquidityPerAsset<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, u128, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		Event1 {},
	}

	#[pallet::error]
	#[cfg_attr(test, derive(PartialEq, Eq))]
	pub enum Error<T> {
		/// Invalid value for a limit. Limit must be non-zero.
		Error1,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set trade volume limit for an asset.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_trade_volume_limit())]
		pub fn asd(origin: OriginFor<T>, asset_id: T::AssetId, trade_volume_limit: (u32, u32)) -> DispatchResult {
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {}

impl<T: Config> XcmDeferFilter<T::RuntimeCall> for Pallet<T> {
	fn deferred_by(
		para: polkadot_parachain::primitives::Id,
		sent_at: polkadot_core_primitives::BlockNumber,
		xcm: &VersionedXcm<T::RuntimeCall>,
	) -> Option<polkadot_core_primitives::BlockNumber> {
		todo!()
	}
}

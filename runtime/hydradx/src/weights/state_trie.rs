// This file is part of HydraDX.

// Copyright (C) 2020-2023  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for `pallet_state_trie_migration`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-01-26, STEPS: `10`, REPEAT: `30`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `bench-bot`, CPU: `Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: `1024`

// Executed Command:
// target/release/hydradx
// benchmark
// pallet
// --chain=dev
// --steps=10
// --repeat=30
// --wasm-execution=compiled
// --heap-pages=4096
// --template=.maintain/pallet-weight-template-no-back.hbs
// --pallet=pallet-state-trie-migration
// --output=weights-1.1.0/state_trie.rs
// --extrinsic=*

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

use pallet_state_trie_migration::weights::WeightInfo;

/// Weights for `pallet_state_trie_migration` using the HydraDX node and recommended hardware.
pub struct HydraWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for HydraWeight<T> {
	/// Storage: `StateTrieMigration::SignedMigrationMaxLimits` (r:1 w:0)
	/// Proof: `StateTrieMigration::SignedMigrationMaxLimits` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `StateTrieMigration::MigrationProcess` (r:1 w:1)
	/// Proof: `StateTrieMigration::MigrationProcess` (`max_values`: Some(1), `max_size`: Some(1042), added: 1537, mode: `MaxEncodedLen`)
	fn continue_migrate() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `141`
		//  Estimated: `2527`
		// Minimum execution time: 23_654_000 picoseconds.
		Weight::from_parts(24_101_000, 2527)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `StateTrieMigration::SignedMigrationMaxLimits` (r:1 w:0)
	/// Proof: `StateTrieMigration::SignedMigrationMaxLimits` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn continue_migrate_wrong_witness() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `109`
		//  Estimated: `1493`
		// Minimum execution time: 7_170_000 picoseconds.
		Weight::from_parts(7_487_000, 1493).saturating_add(T::DbWeight::get().reads(1_u64))
	}
	fn migrate_custom_top_success() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 13_335_000 picoseconds.
		Weight::from_parts(13_596_000, 0)
	}
	/// Storage: UNKNOWN KEY `0x666f6f` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x666f6f` (r:1 w:1)
	fn migrate_custom_top_fail() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `146`
		//  Estimated: `3611`
		// Minimum execution time: 40_202_000 picoseconds.
		Weight::from_parts(40_793_000, 3611)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn migrate_custom_child_success() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 13_441_000 picoseconds.
		Weight::from_parts(13_910_000, 0)
	}
	/// Storage: UNKNOWN KEY `0x666f6f` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x666f6f` (r:1 w:1)
	fn migrate_custom_child_fail() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `106`
		//  Estimated: `3571`
		// Minimum execution time: 39_284_000 picoseconds.
		Weight::from_parts(39_823_000, 3571)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: UNKNOWN KEY `0x6b6579` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x6b6579` (r:1 w:1)
	/// The range of component `v` is `[1, 4194304]`.
	fn process_top_key(v: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `262 + v * (1 ±0)`
		//  Estimated: `3725 + v * (1 ±0)`
		// Minimum execution time: 6_940_000 picoseconds.
		Weight::from_parts(7_084_000, 3725)
			// Standard Error: 2
			.saturating_add(Weight::from_parts(1_372, 0).saturating_mul(v.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
			.saturating_add(Weight::from_parts(0, 1).saturating_mul(v.into()))
	}
}
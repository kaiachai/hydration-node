#![cfg(test)]

use crate::polkadot_test_net::*;
use frame_support::{
	assert_ok,
	sp_runtime::{FixedU128, Permill},
	traits::Contains,
};
use polkadot_xcm::latest::prelude::*;
use polkadot_xcm::VersionedXcm;
use xcm_emulator::TestExt;

#[test]
fn transfer_should_not_work_when_transfering_omnipool_assets_to_omnipool_account() {
	TestNet::reset();

	Hydra::execute_with(|| {
		// Arrange
		let omnipool_account = hydradx_runtime::Omnipool::protocol_account();

		init_omnipool();

		let token_price = FixedU128::from_inner(25_650_000_000_000_000_000);

		assert_ok!(hydradx_runtime::Omnipool::add_token(
			hydradx_runtime::Origin::root(),
			DOT,
			token_price,
			Permill::from_percent(100),
			AccountId::from(BOB),
		));

		// Act & Assert

		// Balances::transfer
		// transfer to Alice should not be filtered
		let successful_call = hydradx_runtime::Call::Balances(pallet_balances::Call::transfer {
			dest: ALICE.into(),
			value: 10 * UNITS,
		});
		let filtered_call = hydradx_runtime::Call::Balances(pallet_balances::Call::transfer {
			dest: omnipool_account.clone(),
			value: 10 * UNITS,
		});

		assert!(hydradx_runtime::CallFilter::contains(&successful_call));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call));

		// Currencies::transfer
		// transfer to Alice should not be filtered
		let successful_call_alice = hydradx_runtime::Call::Currencies(pallet_currencies::Call::transfer {
			dest: ALICE.into(),
			currency_id: DOT,
			amount: 10 * UNITS,
		});
		// transfer of a token that's not registered in omnipool should not be filtered
		let successful_call = hydradx_runtime::Call::Currencies(pallet_currencies::Call::transfer {
			dest: omnipool_account.clone(),
			currency_id: ETH,
			amount: 10 * UNITS,
		});
		let filtered_call_lrna = hydradx_runtime::Call::Currencies(pallet_currencies::Call::transfer {
			dest: omnipool_account.clone(),
			currency_id: LRNA,
			amount: 10 * UNITS,
		});
		let filtered_call_dot = hydradx_runtime::Call::Currencies(pallet_currencies::Call::transfer {
			dest: omnipool_account.clone(),
			currency_id: DOT,
			amount: 10 * UNITS,
		});

		assert!(hydradx_runtime::CallFilter::contains(&successful_call_alice));
		assert!(hydradx_runtime::CallFilter::contains(&successful_call));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_lrna));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_dot));

		// Tokens::transfer
		// transfer to Alice should not be filtered
		let successful_call_alice = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer {
			dest: ALICE.into(),
			currency_id: DOT,
			amount: 10 * UNITS,
		});
		let successful_call = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer {
			dest: omnipool_account.clone(),
			currency_id: ETH,
			amount: 10 * UNITS,
		});
		let filtered_call_lrna = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer {
			dest: omnipool_account.clone(),
			currency_id: LRNA,
			amount: 10 * UNITS,
		});
		let filtered_call_dot = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer {
			dest: omnipool_account.clone(),
			currency_id: DOT,
			amount: 10 * UNITS,
		});

		assert!(hydradx_runtime::CallFilter::contains(&successful_call_alice));
		assert!(hydradx_runtime::CallFilter::contains(&successful_call));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_lrna));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_dot));

		// Tokens::transfer_keep_alive
		// transfer to Alice should not be filtered
		let successful_call_alice = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_keep_alive {
			dest: ALICE.into(),
			currency_id: DOT,
			amount: 10 * UNITS,
		});
		let successful_call = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_keep_alive {
			dest: omnipool_account.clone(),
			currency_id: ETH,
			amount: 10 * UNITS,
		});
		let filtered_call_lrna = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_keep_alive {
			dest: omnipool_account.clone(),
			currency_id: LRNA,
			amount: 10 * UNITS,
		});
		let filtered_call_dot = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_keep_alive {
			dest: omnipool_account.clone(),
			currency_id: DOT,
			amount: 10 * UNITS,
		});

		assert!(hydradx_runtime::CallFilter::contains(&successful_call_alice));
		assert!(hydradx_runtime::CallFilter::contains(&successful_call));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_dot));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_lrna));

		// Tokens::transfer_all
		// transfer to Alice should not be filtered
		let successful_call_alice = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_all {
			dest: ALICE.into(),
			currency_id: DOT,
			keep_alive: true,
		});
		let successful_call = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_all {
			dest: omnipool_account.clone(),
			currency_id: ETH,
			keep_alive: true,
		});
		let filtered_call_lrna = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_all {
			dest: omnipool_account.clone(),
			currency_id: LRNA,
			keep_alive: true,
		});
		let filtered_call_dot = hydradx_runtime::Call::Tokens(orml_tokens::Call::transfer_all {
			dest: omnipool_account,
			currency_id: DOT,
			keep_alive: true,
		});

		assert!(hydradx_runtime::CallFilter::contains(&successful_call_alice));
		assert!(hydradx_runtime::CallFilter::contains(&successful_call));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_lrna));
		assert!(!hydradx_runtime::CallFilter::contains(&filtered_call_dot));
	});
}

#[test]
fn calling_pallet_uniques_extrinsic_should_be_filtered_by_call_filter() {
	TestNet::reset();

	Hydra::execute_with(|| {
		let call = hydradx_runtime::Call::Uniques(pallet_uniques::Call::create {
			collection: 1u128,
			admin: AccountId::from(ALICE),
		});

		assert!(!hydradx_runtime::CallFilter::contains(&call));
	});
}

#[test]
fn calling_pallet_xcm_extrinsic_should_be_filtered_by_call_filter() {
	TestNet::reset();

	Hydra::execute_with(|| {
		// the values here don't need to make sense, all we need is a valid Call
		let call = hydradx_runtime::Call::PolkadotXcm(pallet_xcm::Call::send {
			dest: Box::new(MultiLocation::parent().into()),
			message: Box::new(VersionedXcm::from(Xcm(vec![]))),
		});

		assert!(!hydradx_runtime::CallFilter::contains(&call));
	});
}

#[test]
fn calling_orml_xcm_extrinsic_should_be_filtered_by_call_filter() {
	TestNet::reset();

	Hydra::execute_with(|| {
		// the values here don't need to make sense, all we need is a valid Call
		let call = hydradx_runtime::Call::OrmlXcm(orml_xcm::Call::send_as_sovereign {
			dest: Box::new(MultiLocation::parent().into()),
			message: Box::new(VersionedXcm::from(Xcm(vec![]))),
		});

		assert!(!hydradx_runtime::CallFilter::contains(&call));
	});
}

use super::*;
use crate::math::AssetStateChange;
use crate::types::BalanceUpdate::{Decrease, Increase};
use frame_support::pallet_prelude::*;
use sp_runtime::{FixedPointNumber, FixedU128};
use std::ops::{Add, Deref, Sub};

pub type Price = FixedU128;

#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct AssetState<Balance> {
	/// Quantity of asset in omnipool
	pub(super) reserve: Balance,
	/// Quantity of Hub Asset matching this asset
	pub(super) hub_reserve: Balance,
	/// Quantity of LP shares for this asset
	pub(super) shares: Balance,
	/// Quantity of LP shares for this asset owned by protocol
	pub(super) protocol_shares: Balance,
	/// TVL of asset
	pub(super) tvl: Balance,
}

impl<Balance> AssetState<Balance>
where
	Balance: Into<<FixedU128 as FixedPointNumber>::Inner> + Copy + CheckedAdd + CheckedSub + Default,
{
	/// Calculate price for actual state
	pub(super) fn price(&self) -> FixedU128 {
		FixedU128::from((self.hub_reserve.into(), self.reserve.into()))
	}

	pub(super) fn delta_update(&mut self, delta: &AssetStateChange<Balance>) -> Option<()> {
		self.reserve = update_value!(self.reserve, delta.delta_reserve)?;
		self.hub_reserve = update_value!(self.hub_reserve, delta.delta_hub_reserve)?;
		self.shares = update_value!(self.shares, delta.delta_shares)?;
		self.protocol_shares = update_value!(self.protocol_shares, delta.delta_protocol_shares)?;
		self.tvl = update_value!(self.tvl, delta.delta_tvl)?;
		Some(())
	}
}

/// Position in Omnipool represents a moment when LP provided liquidity of an asset at that moment’s price.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Position<Balance, AssetId> {
	/// Provided Asset
	pub(super) asset_id: AssetId,
	/// Amount of asset added to omnipool
	pub(super) amount: Balance,
	/// Quantity of LP shares owned by LP
	pub(super) shares: Balance,
	/// Price at which liquidity was provided
	pub(super) price: Balance,
}

// Using FixedU128 to represent a price which uses u128 as inner type, so let's convert `Balance` into FixedU128
impl<Balance, AssetId> Position<Balance, AssetId>
where
	Balance: From<u128> + Into<u128> + Copy + CheckedAdd + CheckedSub + Default,
{
	pub(super) fn fixed_price(&self) -> Price {
		Price::from_inner(self.price.into())
	}

	pub(super) fn price_to_balance(price: Price) -> Balance {
		price.into_inner().into()
	}

	pub(super) fn delta_update(
		&mut self,
		delta_reserve: &BalanceUpdate<Balance>,
		delta_shares: &BalanceUpdate<Balance>,
	) -> Option<()> {
		self.amount = update_value!(self.amount, delta_reserve)?;
		self.shares = update_value!(self.shares, delta_shares)?;
		Some(())
	}
}

/// Simple type to represent imbalance which can be positive or negative.
// Note: Simple prefix is used not to confuse with Imbalance trait from frame_support.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub(super) struct SimpleImbalance<Balance: Copy> {
	pub(super) value: Balance,
	pub(super) negative: bool,
}

impl<Balance: Default + Copy> Default for SimpleImbalance<Balance> {
	fn default() -> Self {
		Self {
			value: Balance::default(),
			negative: true,
		}
	}
}

impl<Balance: CheckedAdd + CheckedSub + PartialOrd + Copy> Add<Balance> for SimpleImbalance<Balance> {
	type Output = Option<Self>;

	fn add(self, amount: Balance) -> Self::Output {
		let (value, sign) = if self.is_positive() {
			(self.value.checked_add(&amount)?, self.negative)
		} else if self.value < amount {
			(amount.checked_sub(&self.value)?, false)
		} else {
			(self.value.checked_sub(&amount)?, self.negative)
		};
		Some(Self { value, negative: sign })
	}
}

impl<Balance: CheckedAdd + CheckedSub + PartialOrd + Copy> Sub<Balance> for SimpleImbalance<Balance> {
	type Output = Option<Self>;

	fn sub(self, amount: Balance) -> Self::Output {
		let (value, sign) = if self.is_negative() {
			(self.value.checked_add(&amount)?, self.negative)
		} else if self.value < amount {
			(amount.checked_sub(&self.value)?, true)
		} else {
			(self.value.checked_sub(&amount)?, self.negative)
		};
		Some(Self { value, negative: sign })
	}
}

impl<Balance: CheckedAdd + CheckedSub + PartialOrd + Copy> SimpleImbalance<Balance> {
	pub(super) fn is_negative(&self) -> bool {
		self.negative
	}

	pub(super) fn is_positive(&self) -> bool {
		!self.negative
	}
}

#[derive(Copy, Clone)]
pub(super) enum BalanceUpdate<Balance>
where
	Balance: Default,
{
	Increase(Balance),
	Decrease(Balance),
}

impl<Balance: CheckedAdd + CheckedSub + PartialOrd + Copy + Default> BalanceUpdate<Balance> {
	pub(crate) fn diff(self, other: Self) -> Option<Self> {
		self.checked_add(&other)
	}
}

impl<Balance: CheckedAdd + CheckedSub + PartialOrd + Default> Add<Self> for BalanceUpdate<Balance> {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		match (self, rhs) {
			(Increase(a), Increase(b)) => BalanceUpdate::Increase(a + b),
			(Decrease(a), Decrease(b)) => BalanceUpdate::Decrease(a + b),
			(Increase(a), Decrease(b)) => {
				if a >= b {
					BalanceUpdate::Increase(a - b)
				} else {
					BalanceUpdate::Decrease(b - a)
				}
			}
			(Decrease(a), Increase(b)) => {
				if a >= b {
					BalanceUpdate::Decrease(a - b)
				} else {
					BalanceUpdate::Increase(b - a)
				}
			}
		}
	}
}

impl<Balance: CheckedAdd + CheckedSub + PartialOrd + Copy + Default> CheckedAdd for BalanceUpdate<Balance> {
	fn checked_add(&self, v: &Self) -> Option<Self> {
		match (self, v) {
			(Increase(a), Increase(b)) => Some(BalanceUpdate::Increase(a.checked_add(b)?)),
			(Decrease(a), Decrease(b)) => Some(BalanceUpdate::Decrease(a.checked_add(b)?)),
			(Increase(a), Decrease(b)) => {
				if a >= b {
					Some(BalanceUpdate::Increase(a.checked_sub(b)?))
				} else {
					Some(BalanceUpdate::Increase(b.checked_sub(a)?))
				}
			}
			(Decrease(a), Increase(b)) => {
				if a >= b {
					Some(BalanceUpdate::Decrease(a.checked_sub(b)?))
				} else {
					Some(BalanceUpdate::Increase(b.checked_sub(a)?))
				}
			}
		}
	}
}

impl<Balance: Default> Default for BalanceUpdate<Balance> {
	fn default() -> Self {
		BalanceUpdate::Increase(Balance::default())
	}
}

impl<Balance: Default> Deref for BalanceUpdate<Balance> {
	type Target = Balance;

	fn deref(&self) -> &Self::Target {
		match self {
			Increase(amount) | Decrease(amount) => amount,
		}
	}
}

#[macro_export]
macro_rules! update_value {
	( $x:expr, $y:expr) => {{
		match &$y {
			BalanceUpdate::Increase(amount) => $x.checked_add(&amount),
			BalanceUpdate::Decrease(amount) => $x.checked_sub(&amount),
		}
	}};
}

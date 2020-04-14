use std::cmp::Ordering;
use derive_more::{Add, AddAssign, Sub, SubAssign};
use crate::import::*;
use rust_decimal::prelude::ToPrimitive;

#[derive(Default, Copy, Clone, PartialEq, Hash, Eq, Debug, Serialize, Add, AddAssign, Sub, SubAssign)]
pub struct ResourceProfile {
    pub ipc: Decimal,
    pub memory: u64,
    pub network: u64,
    pub disk: u64,
}

impl ResourceProfile {
    pub fn normalize(&self, other: &ResourceProfile) -> NormalizedResourceProfile {
        NormalizedResourceProfile {
            ipc: self.ipc.normalize_to(&other.ipc),
            memory: self.memory.normalize_to(&other.memory),
            network: self.network.normalize_to(&other.network),
            disk: self.disk.normalize_to(&other.disk),
        }
    }
}

#[derive(Default, Clone, PartialEq, Hash, Eq, Debug, Serialize, Add, AddAssign, Sub, SubAssign)]
pub struct NormalizedResourceProfile {
    pub ipc: Decimal,
    pub memory: Decimal,
    pub network: Decimal,
    pub disk: Decimal,
}

const fn one() -> Decimal {
    Decimal::from_parts(1, 0, 0, false, 0)
}

impl NormalizedResourceProfile {
    pub const MAX: NormalizedResourceProfile = NormalizedResourceProfile {ipc: one(), disk: one(), memory: one(), network: one()};

    pub fn inner_product(&self) -> u64 {
        let product = self.ipc + self.memory + self.network + self.disk;
        product.to_u64().unwrap()
    }

}

impl Ord for NormalizedResourceProfile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner_product().cmp(&other.inner_product())
    }
}

impl PartialOrd for NormalizedResourceProfile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn one() {
        assert_eq!(one(), Decimal::new(1,0));
    }
}

trait DecimalNormalize {
    fn normalize_to(&self, other: &Self) -> Decimal;
}

impl DecimalNormalize for Decimal {
    fn normalize_to(&self, other: &Decimal) -> Decimal {
        debug_assert!(self < other);
        self / other
    }
}

impl DecimalNormalize for u64 {
    fn normalize_to(&self, other: &u64) -> Decimal {
        let num: Decimal = (*self).into();
        let other: Decimal = (*other).into();
        num.normalize_to(&other)
    }
}
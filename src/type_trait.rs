use num_traits::{FromPrimitive, One, ToPrimitive, Zero, float, int};
use std::ops::Neg;

// 浮点泛型
pub trait FloatTrait:
    float::Float
    + Send
    + Sync
    + float::FloatConst
    + FromPrimitive
    + One
    + Zero
    + std::iter::Sum
    + ndarray::ScalarOperand
    + ToPrimitive
    + Neg<Output = Self>
    + std::ops::Mul<Output = Self>
{
}

impl<T> FloatTrait for T where
    T: float::Float
        + Send
        + Sync
        + float::FloatConst
        + FromPrimitive
        + Zero
        + One
        + std::iter::Sum
        + ndarray::ScalarOperand
        + ToPrimitive
        + Neg<Output = Self>
{
}

// 无符号整数泛型
pub trait IntTrait: int::PrimInt + Send + Sync + FromPrimitive + Zero + std::iter::Sum {}

impl<T> IntTrait for T where T: int::PrimInt + Send + Sync + FromPrimitive + Zero + std::iter::Sum {}

//! BGR Color Data Structure and Utility Implementations.

use crate::image::LumaMethod;
use std::ops::{Add, AddAssign, Div};

/// 3-Channel BGR Color Value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bgr<T = u8> {
    pub b: T,
    pub g: T,
    pub r: T,
}

impl<T> Bgr<T> {
    #[inline]
    pub const fn new(b: T, g: T, r: T) -> Self {
        Self { b, g, r }
    }
}

impl<T> From<(T, T, T)> for Bgr<T> {
    #[inline]
    fn from(tuple: (T, T, T)) -> Self {
        Self {
            b: tuple.0,
            g: tuple.1,
            r: tuple.2,
        }
    }
}

impl<T> From<Bgr<T>> for (T, T, T) {
    #[inline]
    fn from(bgr: Bgr<T>) -> Self {
        (bgr.b, bgr.g, bgr.r)
    }
}

impl Bgr<u8> {
    #[inline]
    pub fn from_bgra_slice(slice: &[u8]) -> Self {
        Self {
            b: slice[0],
            g: slice[1],
            r: slice[2],
        }
    }

    #[inline]
    pub fn luma(self, method: LumaMethod) -> u8 {
        method.calculate_luma(self)
    }

    #[inline]
    pub fn average(self) -> u8 {
        ((u32::from(self.b) + u32::from(self.g) + u32::from(self.r)) / 3) as u8
    }

    #[inline]
    pub fn distance_sq_f32(self, other: Self) -> f32 {
        let db = f32::from(self.b) - f32::from(other.b);
        let dg = f32::from(self.g) - f32::from(other.g);
        let dr = f32::from(self.r) - f32::from(other.r);
        db * db + dg * dg + dr * dr
    }

    #[inline]
    pub fn distance_f32(self, other: Self) -> f32 {
        self.distance_sq_f32(other).sqrt()
    }

    #[inline]
    pub fn to_f64(self) -> Bgr<f64> {
        Bgr {
            b: self.b as f64,
            g: self.g as f64,
            r: self.r as f64,
        }
    }

    #[inline]
    pub fn to_u64(self) -> Bgr<u64> {
        Bgr {
            b: self.b as u64,
            g: self.g as u64,
            r: self.r as u64,
        }
    }
}

impl Bgr<f64> {
    #[inline]
    pub fn from_bgra_slice_f64(slice: &[u8]) -> Self {
        Self {
            b: slice[0] as f64,
            g: slice[1] as f64,
            r: slice[2] as f64,
        }
    }

    #[inline]
    pub fn luma(self, method: LumaMethod) -> f64 {
        method.calculate_luma_f64(self)
    }

    #[inline]
    pub fn average(self) -> f64 {
        (self.b + self.g + self.r) / 3.0
    }

    #[inline]
    pub fn max_channel(self) -> f64 {
        self.r.max(self.g).max(self.b)
    }

    #[inline]
    pub fn min_channel(self) -> f64 {
        self.r.min(self.g).min(self.b)
    }

    #[inline]
    pub fn max_channel_diff(self) -> f64 {
        self.max_channel() - self.min_channel()
    }

    #[inline]
    pub fn abs_diff(self, other: Self) -> Self {
        Self {
            b: (self.b - other.b).abs(),
            g: (self.g - other.g).abs(),
            r: (self.r - other.r).abs(),
        }
    }

    #[inline]
    pub fn distance_sq(self, other: Self) -> f64 {
        let db = self.b - other.b;
        let dg = self.g - other.g;
        let dr = self.r - other.r;
        db * db + dg * dg + dr * dr
    }

    #[inline]
    pub fn distance(self, other: Self) -> f64 {
        self.distance_sq(other).sqrt()
    }

    #[inline]
    pub fn sum_channels(self) -> f64 {
        self.b + self.g + self.r
    }
}

impl Add for Bgr<f64> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            b: self.b + rhs.b,
            g: self.g + rhs.g,
            r: self.r + rhs.r,
        }
    }
}

impl AddAssign for Bgr<f64> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.b += rhs.b;
        self.g += rhs.g;
        self.r += rhs.r;
    }
}

impl Div<f64> for Bgr<f64> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            b: self.b / rhs,
            g: self.g / rhs,
            r: self.r / rhs,
        }
    }
}

impl Add for Bgr<u64> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            b: self.b + rhs.b,
            g: self.g + rhs.g,
            r: self.r + rhs.r,
        }
    }
}

impl AddAssign for Bgr<u64> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.b += rhs.b;
        self.g += rhs.g;
        self.r += rhs.r;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgr_average_and_distance() {
        let c1 = Bgr::new(10u8, 20u8, 30u8);
        assert_eq!(c1.average(), 20);

        let c2 = Bgr::new(13u8, 24u8, 34u8);
        let dist = c1.distance_f32(c2);
        assert!((dist - 6.403124).abs() < 1e-4);

        let f1 = Bgr::new(10.0, 20.0, 30.0);
        let f2 = Bgr::new(13.0, 24.0, 34.0);
        assert_eq!(f1.average(), 20.0);
        assert_eq!(f1.max_channel_diff(), 20.0);
        assert!((f1.distance(f2) - 6.4031242374328485).abs() < 1e-6);
    }
}

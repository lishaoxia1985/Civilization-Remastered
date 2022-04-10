#![allow(dead_code)]

use std::array;
use std::cmp::{max, min};
use std::f64::consts::PI;
use std::ops::{Add, Sub};

use bevy::math::{DMat2, DVec2, IVec2};

const SQRT_3: f64 = 1.732050807568877293527446341505872367_f64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hex(IVec2);
impl Hex {
    const HEX_DIRECTIONS: [Self; 6] = [
        Self::new(1, 0),
        Self::new(1, -1),
        Self::new(0, -1),
        Self::new(-1, 0),
        Self::new(-1, 1),
        Self::new(0, 1),
    ];

    const HEX_DIAGONALS: [Self; 6] = [
        Self::new(2, -1),
        Self::new(1, -2),
        Self::new(-1, -1),
        Self::new(-2, 1),
        Self::new(-1, 2),
        Self::new(1, 1),
    ];

    const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    const fn x(self) -> i32 {
        self.0.x
    }

    const fn y(self) -> i32 {
        self.0.y
    }

    const fn z(self) -> i32 {
        -self.0.x - self.0.y
    }

    pub const fn into_inner(self) -> IVec2 {
        self.0
    }

    pub const fn to_array(self) -> [i32; 2] {
        [self.0.x, self.0.y]
    }

    pub fn to_offset_coordinate(
        self,
        offset: Offset,
        orientation: HexOrientation,
    ) -> OffsetCoordinate {
        match orientation {
            HexOrientation::Pointy => {
                let col: i32 = self.0.x + (self.0.y + offset.value() * (self.0.y & 1)) / 2;
                let row: i32 = self.0.y;
                OffsetCoordinate::new(col, row)
            }
            HexOrientation::Flat => {
                let col: i32 = self.0.x;
                let row: i32 = self.0.y + (self.0.x + offset.value() * (self.0.x & 1)) / 2;
                OffsetCoordinate::new(col, row)
            }
        }
    }

    pub fn to_doubled_coordinate(self, orientation: HexOrientation) -> DoubledCoordinate {
        match orientation {
            HexOrientation::Pointy => {
                let col: i32 = 2 * self.0.x + self.0.y;
                let row: i32 = self.0.y;
                DoubledCoordinate::new(col, row)
            }
            HexOrientation::Flat => {
                let col: i32 = self.0.x;
                let row: i32 = 2 * self.0.y + self.0.x;
                DoubledCoordinate::new(col, row)
            }
        }
    }

    pub fn hex_neighbor(self, direction: i32) -> Hex {
        Self(self.0 + Self::HEX_DIRECTIONS[direction as usize].0)
    }

    pub fn hex_diagonal_neighbor(self, direction: i32) -> Hex {
        Self(self.0 + Self::HEX_DIAGONALS[direction as usize].0)
    }

    pub fn hex_length(self) -> i32 {
        (self.0.x.abs() + self.0.y.abs() + self.z().abs()) / 2
    }

    pub fn hex_distance(a: Self, b: Self) -> i32 {
        Self(a.0 - b.0).hex_length()
    }

    /// Return a `Vec<Hex>` containing all [`Hex`] which are exactly at a given `distance` from `self`.
    /// If `distance` = 0 the `Vec<Hex>` will be empty. \
    /// The number of returned hexes is equal to `6 * distance`.
    pub fn hexes_at_distance(self, distance: u32) -> Vec<Hex> {
        let mut hex_list = Vec::with_capacity((6 * distance) as usize);
        let radius = distance as i32;
        let mut hex = Hex(self.0 + Self::HEX_DIRECTIONS[4].0 * radius);
        for i in 0..6 {
            for _ in 0..radius {
                hex_list.push(hex);
                hex = hex.hex_neighbor(i);
            }
        }
        hex_list
    }

    /// Return a `Vec<Hex>` containing all [`Hex`] around `self` in a given `distance`, including `self`. \
    /// The number of returned hexes is equal to `3 * distance * (distance + 1) + 1`.
    pub fn hexes_in_distance(self, distance: u32) -> Vec<Hex> {
        let mut hex_list = Vec::with_capacity((3 * distance * (distance + 1) + 1) as usize);
        let radius = distance as i32;
        for q in -radius..=radius {
            for r in max(-radius, -q - radius)..=min(radius, -q + radius) {
                let hex = Hex(self.0 + IVec2::new(q, r));
                hex_list.push(hex);
            }
        }
        hex_list
    }

    pub fn hex_rotate_left(self) -> Self {
        Self(-IVec2::new(self.z(), self.0.x))
    }

    pub fn hex_rotate_right(self) -> Self {
        Self(-IVec2::new(self.0.y, self.z()))
    }

    /// Rounds floating point coordinates to [`Hex`].
    pub fn round(fractional_hex: DVec2) -> Self {
        let mut rounded = fractional_hex.round();

        let diff = fractional_hex - rounded;

        if diff.x.abs() >= diff.y.abs() {
            rounded.x += 0.5_f64.mul_add(diff.y, diff.x).round();
        } else {
            rounded.y += 0.5_f64.mul_add(diff.x, diff.y).round();
        }

        Self(rounded.as_ivec2())
    }
}

impl Add for Hex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Hex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<[i32; 2]> for Hex {
    #[inline]
    fn from(a: [i32; 2]) -> Self {
        Self(a.into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OffsetCoordinate(pub IVec2);
impl OffsetCoordinate {
    pub fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    pub fn to_hex(self, offset: Offset, orientation: HexOrientation) -> Hex {
        match orientation {
            HexOrientation::Pointy => {
                let q: i32 = self.0.x - (self.0.y + offset.value() * (self.0.y & 1)) / 2;
                let r: i32 = self.0.y;
                Hex::new(q, r)
            }
            HexOrientation::Flat => {
                let q: i32 = self.0.x;
                let r: i32 = self.0.y - (self.0.x + offset.value() * (self.0.x & 1)) / 2;
                Hex::new(q, r)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DoubledCoordinate(IVec2);
impl DoubledCoordinate {
    pub fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    pub fn to_hex(self, orientation: HexOrientation) -> Hex {
        match orientation {
            HexOrientation::Pointy => {
                let q: i32 = (self.0.x - self.0.y) / 2;
                let r: i32 = self.0.y;
                Hex::new(q, r)
            }
            HexOrientation::Flat => {
                let q: i32 = self.0.x;
                let r: i32 = (self.0.y - self.0.x) / 2;
                Hex::new(q, r)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Orientation {
    /// Matrix used to compute hexagonal coordinates to pixel coordinates
    pub f: DMat2,
    /// Matrix used to compute pixel coordinates to hexagonal coordinates
    pub b: DMat2,
    pub start_angle: f64,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    NoDirection,
}

impl Direction {
    // Pointy hex edge or Flat hex corner direction
    pub const ARRAY_1: [Direction; 6] = [
        Direction::East,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
        Direction::NorthEast,
    ];
    // Flat hex edge or Pointy hex corner direction
    pub const ARRAY_2: [Direction; 6] = [
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::NorthWest,
        Direction::North,
    ];
    pub fn opposite_direction(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::NorthEast => Direction::SouthWest,
            Direction::East => Direction::West,
            Direction::SouthEast => Direction::NorthWest,
            Direction::South => Direction::North,
            Direction::SouthWest => Direction::NorthEast,
            Direction::West => Direction::East,
            Direction::NorthWest => Direction::SouthEast,
            Direction::NoDirection => panic!("This direction has no opposite direction."),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HexLayout {
    pub orientation: HexOrientation,
    pub size: DVec2,
    pub origin: DVec2,
}
impl HexLayout {
    pub const fn edge_direction(&self) -> [Direction; 6] {
        match self.orientation {
            HexOrientation::Pointy => Direction::ARRAY_1,
            HexOrientation::Flat => Direction::ARRAY_2,
        }
    }

    pub const fn corner_direction(&self) -> [Direction; 6] {
        match self.orientation {
            HexOrientation::Pointy => Direction::ARRAY_2,
            HexOrientation::Flat => Direction::ARRAY_1,
        }
    }

    pub fn hex_to_pixel(self, hex: Hex) -> DVec2 {
        let m = self.orientation.value();
        let size: DVec2 = self.size;
        let origin: DVec2 = self.origin;
        let mat2 = m.f;
        let pixel_position = mat2 * (hex.0.as_dvec2()) * size;
        pixel_position + origin
    }

    pub fn pixel_to_hex(self, pixel_position: DVec2) -> Hex {
        let m = self.orientation.value();
        let (size, origin) = (self.size, self.origin);
        let pt = (pixel_position - origin) / size;
        let mat2 = m.b;
        let fractional_hex = mat2 * pt;
        Hex::round(fractional_hex)
    }

    pub fn polygon_corner(self, hex: Hex, i: i32) -> DVec2 {
        let center: DVec2 = self.hex_to_pixel(hex);
        let offset: DVec2 = self.hex_corner_offset(i);
        center + offset
    }

    pub fn polygon_corners(self, hex: Hex) -> [DVec2; 6] {
        array::from_fn(|i| self.polygon_corner(hex, i as i32))
    }

    fn hex_corner_offset(self, corner: i32) -> DVec2 {
        let m = self.orientation.value();
        let size: DVec2 = self.size;
        let angle: f64 = 2.0 * PI * (m.start_angle - corner as f64) / 6.0;
        size * DVec2::from_angle(angle)
    }
}

pub fn hex_linedraw(a: Hex, b: Hex) -> Vec<Hex> {
    let n: i32 = Hex::hex_distance(a, b);
    let a_nudge = a.0.as_dvec2() + DVec2::new(1e-06, 1e-06);
    let b_nudge = b.0.as_dvec2() + DVec2::new(1e-06, 1e-06);
    let step: f64 = 1.0 / max(n, 1) as f64;
    (0..=n)
        .map(|i| Hex::round(a_nudge.lerp(b_nudge, step * i as f64)))
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub enum Offset {
    Even,
    Odd,
}

impl Offset {
    fn value(self) -> i32 {
        match self {
            Self::Even => 1,
            Self::Odd => -1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum HexOrientation {
    /// ⬢, [`Hex`] is pointy-topped
    Pointy,
    /// ⬣, [`Hex`] is flat-topped
    Flat,
}

impl HexOrientation {
    const POINTY_ORIENTATION: Orientation = Orientation {
        f: DMat2::from_cols_array(&[SQRT_3, 0.0, SQRT_3 / 2.0, 3.0 / 2.0]),
        b: DMat2::from_cols_array(&[SQRT_3 / 3.0, 0.0, -1.0 / 3.0, 2.0 / 3.0]),
        start_angle: 0.5,
    };

    const FLAT_ORIENTATION: Orientation = Orientation {
        f: DMat2::from_cols_array(&[3.0 / 2.0, SQRT_3 / 2.0, 0.0, SQRT_3]),
        b: DMat2::from_cols_array(&[2.0 / 3.0, -1.0 / 3.0, 0.0, SQRT_3 / 3.0]),
        start_angle: 0.0,
    };

    fn value(self) -> Orientation {
        match self {
            Self::Pointy => Self::POINTY_ORIENTATION,
            Self::Flat => Self::FLAT_ORIENTATION,
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use bevy::math::{DVec2, IVec2};

    use super::{
        hex_linedraw, DoubledCoordinate, Hex, HexLayout, HexOrientation, Offset, OffsetCoordinate,
    };

    pub fn equal_hex(name: &str, a: Hex, b: Hex) {
        if a != b {
            panic!("FAIL {}", name);
        }
    }

    pub fn equal_offset_coordinate(name: &str, a: OffsetCoordinate, b: OffsetCoordinate) {
        if a != b {
            panic!("FAIL {}", name);
        }
    }

    pub fn equal_doubled_coordinate(name: &str, a: DoubledCoordinate, b: DoubledCoordinate) {
        if a != b {
            panic!("FAIL {}", name);
        }
    }

    pub fn equal_hex_array(name: &str, a: Vec<Hex>, b: Vec<Hex>) {
        assert_eq!(a.len(), b.len(), "{}", format!("FAIL {}", name));
        for (x, y) in a.into_iter().zip(b.into_iter()) {
            equal_hex(name, x, y);
        }
    }

    #[test]
    pub fn test_hex_neighbor() {
        equal_hex(
            "hex_neighbor",
            Hex::new(1, -3),
            Hex::new(1, -2).hex_neighbor(2),
        );
    }

    #[test]
    pub fn test_hex_diagonal() {
        equal_hex(
            "hex_diagonal",
            Hex::new(-1, -1),
            Hex::new(1, -2).hex_diagonal_neighbor(3),
        );
    }

    #[test]
    pub fn test_hex_distance() {
        assert_eq!(
            7,
            Hex::hex_distance(Hex::new(3, -7), Hex(IVec2::ZERO)),
            "FAIL hex_distance"
        );
    }

    #[test]
    pub fn test_hex_rotate_right() {
        equal_hex(
            "hex_rotate_right",
            Hex::new(1, -3).hex_rotate_right(),
            Hex::new(3, -2),
        );
    }

    #[test]
    pub fn test_hex_rotate_left() {
        equal_hex(
            "hex_rotate_left",
            Hex::new(1, -3).hex_rotate_left(),
            Hex::new(-2, -1),
        );
    }

    #[test]
    pub fn test_hex_round() {
        let a = DVec2::ZERO;
        let b = DVec2::new(1.0, -1.0);
        let c = DVec2::new(0.0, -1.0);
        equal_hex(
            "hex_round 1",
            Hex::new(5, -10),
            Hex::round(DVec2::ZERO.lerp(DVec2::new(10.0, -20.0), 0.5)),
        );
        equal_hex("hex_round 2", Hex::round(a), Hex::round(a.lerp(b, 0.499)));
        equal_hex("hex_round 3", Hex::round(b), Hex::round(a.lerp(b, 0.501)));
        equal_hex(
            "hex_round 4",
            Hex::round(a),
            Hex::round(a * 0.4 + b * 0.3 + c * 0.3),
        );
        equal_hex(
            "hex_round 5",
            Hex::round(c),
            Hex::round(a * 0.3 + b * 0.3 + c * 0.4),
        );
    }

    #[test]
    pub fn test_hex_linedraw() {
        equal_hex_array(
            "hex_linedraw",
            vec![
                Hex(IVec2::ZERO),
                Hex::new(0, -1),
                Hex::new(0, -2),
                Hex::new(1, -3),
                Hex::new(1, -4),
                Hex::new(1, -5),
            ],
            hex_linedraw(Hex(IVec2::ZERO), Hex::new(1, -5)),
        );
    }

    #[test]
    pub fn test_layout() {
        let h = Hex::new(3, 4);
        let flat: HexLayout = HexLayout {
            orientation: HexOrientation::Flat,
            size: DVec2 { x: 10.0, y: 15.0 },
            origin: DVec2 { x: 35.0, y: 71.0 },
        };
        equal_hex("layout", h, flat.pixel_to_hex(flat.hex_to_pixel(h)));
        let pointy: HexLayout = HexLayout {
            orientation: HexOrientation::Pointy,
            size: DVec2 { x: 10.0, y: 15.0 },
            origin: DVec2 { x: 35.0, y: 71.0 },
        };
        equal_hex("layout", h, pointy.pixel_to_hex(pointy.hex_to_pixel(h)));
    }

    #[test]
    pub fn test_offset_roundtrip() {
        let a = Hex::new(3, 4);
        let b = OffsetCoordinate::new(1, -3);
        equal_hex(
            "conversion_roundtrip even-q",
            a,
            a.to_offset_coordinate(Offset::Even, HexOrientation::Flat)
                .to_hex(Offset::Even, HexOrientation::Flat),
        );
        equal_offset_coordinate(
            "conversion_roundtrip even-q",
            b,
            b.to_hex(Offset::Even, HexOrientation::Flat)
                .to_offset_coordinate(Offset::Even, HexOrientation::Flat),
        );
        equal_hex(
            "conversion_roundtrip odd-q",
            a,
            a.to_offset_coordinate(Offset::Odd, HexOrientation::Flat)
                .to_hex(Offset::Odd, HexOrientation::Flat),
        );
        equal_offset_coordinate(
            "conversion_roundtrip odd-q",
            b,
            b.to_hex(Offset::Odd, HexOrientation::Flat)
                .to_offset_coordinate(Offset::Odd, HexOrientation::Flat),
        );
        equal_hex(
            "conversion_roundtrip even-r",
            a,
            a.to_offset_coordinate(Offset::Even, HexOrientation::Pointy)
                .to_hex(Offset::Even, HexOrientation::Pointy),
        );
        equal_offset_coordinate(
            "conversion_roundtrip even-r",
            b,
            b.to_hex(Offset::Even, HexOrientation::Pointy)
                .to_offset_coordinate(Offset::Even, HexOrientation::Pointy),
        );
        equal_hex(
            "conversion_roundtrip odd-r",
            a,
            a.to_offset_coordinate(Offset::Odd, HexOrientation::Pointy)
                .to_hex(Offset::Odd, HexOrientation::Pointy),
        );
        equal_offset_coordinate(
            "conversion_roundtrip odd-r",
            b,
            b.to_hex(Offset::Odd, HexOrientation::Pointy)
                .to_offset_coordinate(Offset::Odd, HexOrientation::Pointy),
        );
    }

    #[test]
    pub fn test_offset_from_hex() {
        equal_offset_coordinate(
            "offset_from_hex even-q",
            OffsetCoordinate::new(1, 3),
            Hex::new(1, 2).to_offset_coordinate(Offset::Even, HexOrientation::Flat),
        );
        equal_offset_coordinate(
            "offset_from_hex odd-q",
            OffsetCoordinate::new(1, 2),
            Hex::new(1, 2).to_offset_coordinate(Offset::Odd, HexOrientation::Flat),
        );
    }

    #[test]
    pub fn test_offset_to_hex() {
        equal_hex(
            "offset_to_hex even-q",
            Hex::new(1, 2),
            OffsetCoordinate::new(1, 3).to_hex(Offset::Even, HexOrientation::Flat),
        );
        equal_hex(
            "offset_to_hex odd-q",
            Hex::new(1, 2),
            OffsetCoordinate::new(1, 2).to_hex(Offset::Odd, HexOrientation::Flat),
        );
    }

    #[test]
    pub fn test_doubled_roundtrip() {
        let a = Hex::new(3, 4);
        let b = DoubledCoordinate::new(1, -3);
        equal_hex(
            "conversion_roundtrip doubled-q",
            a,
            a.to_doubled_coordinate(HexOrientation::Flat)
                .to_hex(HexOrientation::Flat),
        );
        equal_doubled_coordinate(
            "conversion_roundtrip doubled-q",
            b,
            b.to_hex(HexOrientation::Flat)
                .to_doubled_coordinate(HexOrientation::Flat),
        );
        equal_hex(
            "conversion_roundtrip doubled-r",
            a,
            a.to_doubled_coordinate(HexOrientation::Pointy)
                .to_hex(HexOrientation::Pointy),
        );
        equal_doubled_coordinate(
            "conversion_roundtrip doubled-r",
            b,
            b.to_hex(HexOrientation::Pointy)
                .to_doubled_coordinate(HexOrientation::Pointy),
        );
    }

    #[test]
    pub fn test_doubled_from_hex() {
        equal_doubled_coordinate(
            "doubled_from_hex doubled-q",
            DoubledCoordinate::new(1, 5),
            Hex::new(1, 2).to_doubled_coordinate(HexOrientation::Flat),
        );
        equal_doubled_coordinate(
            "doubled_from_hex doubled-r",
            DoubledCoordinate::new(4, 2),
            Hex::new(1, 2).to_doubled_coordinate(HexOrientation::Pointy),
        );
    }

    #[test]
    pub fn test_doubled_to_hex() {
        equal_hex(
            "doubled_to_hex doubled-q",
            Hex::new(1, 2),
            DoubledCoordinate::new(1, 5).to_hex(HexOrientation::Flat),
        );
        equal_hex(
            "doubled_to_hex doubled-r",
            Hex::new(1, 2),
            DoubledCoordinate::new(4, 2).to_hex(HexOrientation::Pointy),
        );
    }
}

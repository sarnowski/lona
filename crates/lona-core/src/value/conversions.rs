// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Convenience conversions for Value.

use super::Value;
use crate::symbol;

#[cfg(feature = "alloc")]
use super::Function;
#[cfg(feature = "alloc")]
use crate::integer::Integer;
#[cfg(feature = "alloc")]
use crate::list::List;
#[cfg(feature = "alloc")]
use crate::map::Map;
#[cfg(feature = "alloc")]
use crate::ratio::Ratio;
#[cfg(feature = "alloc")]
use crate::string::HeapStr;
#[cfg(feature = "alloc")]
use crate::vector::Vector;

impl From<bool> for Value {
    #[inline]
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[cfg(feature = "alloc")]
impl From<i64> for Value {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Integer(Integer::from_i64(value))
    }
}

#[cfg(not(feature = "alloc"))]
impl From<i64> for Value {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

#[cfg(feature = "alloc")]
impl From<i32> for Value {
    #[inline]
    fn from(value: i32) -> Self {
        Self::Integer(Integer::from(value))
    }
}

#[cfg(not(feature = "alloc"))]
impl From<i32> for Value {
    #[inline]
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

#[cfg(feature = "alloc")]
impl From<Integer> for Value {
    #[inline]
    fn from(value: Integer) -> Self {
        Self::Integer(value)
    }
}

#[cfg(feature = "alloc")]
impl From<Ratio> for Value {
    #[inline]
    fn from(value: Ratio) -> Self {
        Self::Ratio(value)
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<symbol::Id> for Value {
    #[inline]
    fn from(id: symbol::Id) -> Self {
        Self::Symbol(id)
    }
}

#[cfg(feature = "alloc")]
impl From<HeapStr> for Value {
    #[inline]
    fn from(string: HeapStr) -> Self {
        Self::String(string)
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for Value {
    #[inline]
    fn from(text: &str) -> Self {
        Self::String(HeapStr::new(text))
    }
}

#[cfg(feature = "alloc")]
impl From<List> for Value {
    #[inline]
    fn from(list: List) -> Self {
        Self::List(list)
    }
}

#[cfg(feature = "alloc")]
impl From<Vector> for Value {
    #[inline]
    fn from(vector: Vector) -> Self {
        Self::Vector(vector)
    }
}

#[cfg(feature = "alloc")]
impl From<Map> for Value {
    #[inline]
    fn from(map: Map) -> Self {
        Self::Map(map)
    }
}

#[cfg(feature = "alloc")]
impl From<Function> for Value {
    #[inline]
    fn from(func: Function) -> Self {
        Self::Function(func)
    }
}

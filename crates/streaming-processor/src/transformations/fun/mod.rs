//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

pub mod trait_def;

pub mod lowercase;
pub mod replace;
pub mod substring;
pub mod trim;
pub mod uppercase;

pub use trait_def::*;

pub use lowercase::*;
pub use replace::*;
pub use substring::*;
pub use trim::*;
pub use uppercase::*;
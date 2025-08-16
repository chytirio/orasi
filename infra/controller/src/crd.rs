//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! Custom Resource Definition generation for the Orasi controller

use crate::Document;
use kube::CustomResourceExt;

/// Generate the CRD for Document resources
pub fn generate_crd() -> String {
    let crd = Document::crd();
    serde_yaml::to_string(&crd).expect("Failed to serialize CRD")
}

/// Print the CRD to stdout
pub fn print_crd() {
    println!("{}", generate_crd());
}

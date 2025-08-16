//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0

//! CRD generation binary for the Orasi controller

fn main() {
    println!("apiVersion: apiextensions.k8s.io/v1");
    println!("kind: CustomResourceDefinition");
    println!("metadata:");
    println!("  name: documents.orasi.io");
    println!("spec:");
    println!("  group: orasi.io");
    println!("  names:");
    println!("    kind: Document");
    println!("    listKind: DocumentList");
    println!("    plural: documents");
    println!("    singular: document");
    println!("  scope: Namespaced");
    println!("  versions:");
    println!("  - name: v1alpha1");
    println!("    served: true");
    println!("    storage: true");
    println!("    schema:");
    println!("      openAPIV3Schema:");
    println!("        type: object");
    println!("        properties:");
    println!("          spec:");
    println!("            type: object");
    println!("            properties:");
    println!("              content:");
    println!("                type: string");
    println!("              hidden:");
    println!("                type: boolean");
    println!("              metadata:");
    println!("                type: object");
    println!("                properties:");
    println!("                  title:");
    println!("                    type: string");
    println!("                  author:");
    println!("                    type: string");
    println!("                  tags:");
    println!("                    type: array");
    println!("                    items:");
    println!("                      type: string");
}

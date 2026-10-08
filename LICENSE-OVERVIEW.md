# domlet: Technical licensing overview

> **Status:** 2026-10-08  
> **Scope:** repository development state of `domlet` 0.3.0 before its first
> release (commit `3122217`).  
> **Project license:** [MIT](LICENSE-MIT).

This document describes technical findings and provides general background
information. It is not a license grant, legal advice, legal approval, or a
guarantee against claims. The rights granted for `domlet` are defined by
`LICENSE-MIT`, not by this overview. Third-party software remains subject to
its own license terms.

This overview is intended for individuals and companies evaluating
`domlet` for applications or products. It explains the technical distinction
from the official Slint framework, typical use cases, and the checks that remain
necessary for a specific product. The legal discussion focuses on Germany and
the European Union; other jurisdictions may reach different conclusions.

`domlet` is an independent project. It is not affiliated with, sponsored by,
or endorsed by Slint or SixtyFPS GmbH. References to Slint are descriptive and
do not grant any rights to Slint software, names, or branding.

## 1. Summary

**Using the `.slint` language is not the same as using the official Slint
framework.** The reviewed browser build of `domlet` uses its own parser and
a DOM runtime. Its dependency tree contains no official Slint packages.

**Technical conclusion:** The reviewed browser path does not depend on the
official Slint compiler, runtime, or renderer. If the implementation and its
assets are independently created and no Slint software is incorporated, the
use of a compatible input syntax alone is technically distinct from using the
Slint framework. Whether a particular product creates contractual, copyright,
trademark, patent, or other obligations still depends on its actual contents,
use, presentation, agreements, and applicable law.

This distinction can also be relevant to company products and browser-based
operation of an embedded device. It arises from the reviewed implementation,
not from an embedded exception or a general exemption created by HTML or WASM.

The technical basis has been reviewed; the complete origin of the code and the
specific product as a whole have not yet been audited. The legal basis and
limitations are described in sections 3 and 6.

### Key conclusion

> The reviewed `domlet` browser product path does not use the Slint
> framework. It uses an independent implementation of part of the Slint
> language. The reviewed production dependencies contain no official Slint
> packages. This technical finding does not determine the legal obligations of
> every application. Before shipping a product, the separation must be
> confirmed through origin, dependency, artifact, and product-specific legal
> checks.

This assessment does not rule out possible claims and does not mean that all
Slint rights are categorically inapplicable.

## 2. Technical evidence from the project

The reviewed product path is:

```text
own .slint file
    → domlet-macros: parser + Rust code generator
    → domlet + browser APIs
    → WASM creates an HTML DOM in the browser
```

| Review question | Finding in the reviewed development state | Evidence |
|---|---|---|
| Is the official Slint compiler required for the browser product path? | No. The macro reads the UI file and calls the parser and generator from `domlet-macros`. | [macros/src/lib.rs](macros/src/lib.rs), [parser](macros/src/parser.rs), [generator](macros/src/generate.rs) |
| Is the official Slint renderer used? | The reviewed DOM path creates elements through browser APIs, in particular `Document::create_element`. | [src/dom.rs](src/dom.rs) |
| Does `import … from "std-widgets.slint"` load Slint code? | No. The parser checks widget names and the import string; it does not load an official widget file. | [macros/src/parser.rs](macros/src/parser.rs), method `component` |
| Does the browser dependency tree contain official Slint packages? | In the reviewed tree with all features: no `slint`, `slint-build`, or `i-slint-…`. `domlet` and `domlet-macros` belong to the independent project. | Checked with `cargo tree`; see section 7 |
| Which license do `domlet` and `domlet-macros` declare? | MIT. Subject to its conditions, the license also permits sale, modification, and redistribution. | [Cargo.toml](Cargo.toml), [macros/Cargo.toml](macros/Cargo.toml), [LICENSE-MIT](LICENSE-MIT) |
| Is official Slint used anywhere in the repository? | Yes. `tests/slint-compat` uses `slint-build` in a separate workspace. The generated Rust code is not included there. | [test manifest](tests/slint-compat/Cargo.toml), [build.rs](tests/slint-compat/build.rs), [test library](tests/slint-compat/src/lib.rs) |

**Limit of this statement:** A separate parser and a clean dependency tree
demonstrate technical separation, but they do not prove the origin of every
line of code without gaps. The README statement “contains no Slint code” is a
project declaration, not an independent origin audit.

## 3. Basis for distinguishing compatibility from framework use

### A. Ideas and interfaces must be distinguished from specific program code

§ 69a(2) of the German Copyright Act distinguishes protected forms of
expression of a computer program from the underlying ideas and principles,
including those underlying interfaces. This is relevant to the distinction
between compatible functionality and copied program code, but its application
depends on the facts of each case.
Source: [§ 69a UrhG](https://www.gesetze-im-internet.de/urhg/__69a.html).

### B. The CJEU expressly addresses programming languages

In its judgment of 2 May 2012, C-406/10, *SAS Institute v World Programming*,
the CJEU held that functionality, a programming language, and certain data-file
formats are not protected forms of expression of a computer program under the
software copyright rules considered in that case. The judgment is relevant to
independent compatible implementations, but it is not a judgment about this
project. The copying of protected content, such as material from manuals or
source code, must still be assessed separately. The judgment does not create a
blanket exemption from contracts or other intellectual-property rights.
Sources: [CJEU judgment C-406/10](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:62010CJ0406),
[official CJEU summary](https://curia.europa.eu/jcms/upload/docs/application/pdf/2012-05/cp120053en.pdf).

### C. Slint's licensing options apply to its software

Slint distinguishes between the framework, including language tooling,
MIT-licensed documentation and examples, and separately licensed third-party
components. Its published options include GPLv3, royalty-free, and commercial
terms. The royalty-free terms published at the time of review exclude use of
the Slint software within embedded systems. Terms can change and must be
checked for the version and use actually involved.
Sources reviewed on 2026-10-06:
[official Slint terms](https://slint.dev/terms-and-conditions) and
[official repository license overview](https://github.com/slint-ui/slint/blob/master/LICENSE.md).

**Technical assessment in this overview, not confirmation by Slint or legal
advice:** Reading a user's own `.slint` files with an independent
implementation does not technically make that implementation the official
framework. Conversely, copied framework code does not become independently
licensed merely because it is renamed, translated into HTML, or compiled to
WASM.

## 4. Companies, sales, and embedded systems: typical use cases

The following table is a practical screening guide, not a legal determination.
It concerns the reviewed technical distinction from Slint framework use, not
all obligations of a market-ready product. Each result must be confirmed for
the actual application and its dependencies.

| Use case | Assessment | Consequence |
|---|---|---|
| A company uses `domlet` internally | The MIT License for `domlet` does not distinguish between individuals and companies. | Comply with the MIT notice requirement and review other dependencies separately. |
| A company sells an application containing `domlet` | Commercial distribution is permitted under the MIT License, subject to its conditions. | MIT alone does not require disclosure of proprietary source code; include the required copyright and license notice. |
| An embedded device serves a `domlet` web UI to a browser | The hardware context alone does not establish that the official Slint framework is used. | Do not infer a Slint fee or exemption from the input syntax alone; review the implementation, firmware, server, and browser bundle separately. |
| A device has a display but does not use the Slint framework | A display alone is not evidence that Slint software is used. | Review technical suitability separately: `domlet` is a browser DOM solution, not a native microcontroller renderer. |
| The same UI file is also built as a desktop application with the official Slint framework | This is a separate software path involving official Slint software. | Select an applicable Slint license and comply with its conditions. |
| A proprietary embedded UI uses the official Slint renderer | The royalty-free terms published at the time of review exclude embedded-system use. | Review the current commercial and GPLv3 options and obtain product-specific advice. |
| The browser UI actually contains official Slint WASM | Delivery through WASM or a browser does not remove the need to identify and license incorporated software. | Assess the specific integration under the current official terms; do not assume that the delivery format determines the result. |

When official Slint software is actually used, its applicable version and
license terms must be reviewed directly. A `domlet` license does not grant
rights for any additional use of Slint software. See the
[official Slint terms](https://slint.dev/terms-and-conditions).

## 5. Publication through GitHub, crates.io, and other channels

The publication channel does not determine which license applies and does not
verify that the publisher owns all necessary rights. The same origin,
dependency, notice, and trademark checks remain necessary whether the project
is distributed through GitHub, crates.io, docs.rs, a company server, an npm
package, a firmware image, or another channel.

- **GitHub source repository:** Repository publication exposes the project
  source, manifests, documentation, and compatibility-test setup. It does not
  by itself incorporate downloaded dependencies into this repository.
- **crates.io package:** The package archive is the relevant distribution unit.
  The reviewed `cargo package --list` output for `domlet` and
  `domlet-macros` did not contain the separate Slint compatibility-test
  workspace or official Slint packages. Repeat this check for every release.
- **docs.rs:** Documentation is built from the published crate and its declared
  dependencies. Keep public API documentation and package metadata consistent
  with the actual crate contents.
- **WASM, npm, firmware, installers, and product images:** These are separate
  artifacts. Inspect their bundled code, assets, notices, and dependency lists;
  a clean Rust source dependency tree alone is not sufficient evidence.

Copies or substantial portions of `domlet` must retain the copyright and
MIT permission notice as required by [LICENSE-MIT](LICENSE-MIT). This overview
does not impose additional license conditions.

## 6. Limits of this assessment

- **Official compiler in tests:** The licenses of the Slint components apply
  to `tests/slint-compat`. Its separation from the product has been
  demonstrated, but “only a test” is not a license exemption. Permitted tool
  usage and any distribution of test artifacts must be reviewed separately.
- **Code and assets:** Review any copied Slint sources, examples, fonts,
  images, and templates individually for origin and license. A missing Cargo
  entry does not detect copied files.
- **Name and public presentation:** The technical separation does not resolve
  trademark questions concerning the project name or its references to Slint.
  Do not imply an official relationship; the README disclaimer alone is not
  trademark clearance.
- **Other legal questions:** Existing contracts, patents, and competition-law
  questions are outside the scope of this technical review. The legal argument
  in this document concerns Germany and the EU.
- **Complete product:** This overview is not a complete licensing and shipping
  audit of a specific application or embedded firmware. Additional libraries
  and product components must be reviewed separately for the particular
  combination being shipped.

## 7. Verification and approval steps

### Scope and independent verification

The initial technical review was performed on 2026-10-05. The dependency and
package checks below were repeated on 2026-10-08 on commit `3122217` (`domlet`
0.3.0 before its first release).
This is not a reproducible certification of every `domlet` release. For each
release or product, repeat the review on a clean, identified revision with the
lockfiles, enabled features, target platform, and shipped artifacts that will
actually be used.

The following command was run successfully in the project directory without
modifying the lockfile and can be used there to repeat the check:

```powershell
cargo tree --locked --offline -p domlet --all-features --target wasm32-unknown-unknown --edges normal,build --prefix none
```

For `--offline`, the required package information must already be available in
the Cargo cache. In an application, review its complete dependency tree as
well, not only the `domlet` package.

Documented result: no official Slint packages were found in the reviewed tree.
The macro entry point, import handling, DOM creation, and separate compatibility
test were also inspected. This was not a complete source-code, binary, or
license audit of every dependency.

### Before final product approval

1. **Record the release:** Clearly document the reviewed commit, lockfiles,
   features, target platforms, and shipped files.
2. **Identify every product component:** Review browser/WASM, server, and the
   actual firmware separately, including non-Cargo files and build outputs.
3. **Confirm origin:** Review the parser, generator, runtime, and assets for
   incorporated material and document the results.
4. **Prepare license evidence:** Create a component list with versions and
   license texts; ship required copyright and NOTICE files.
5. **Separate test tooling:** Document the licensing basis for the official
   compiler and do not accidentally include its test output in the product.
6. **Obtain legal review where appropriate:** Have this evidence and the
   planned public presentation reviewed by a qualified specialist in IT and
   open-source law before relying on it for a commercial product.

## 8. Conclusion

**The separation of the independent DOM product path from the official Slint
framework is the central condition underlying this assessment.**

The available technical findings document that the reviewed DOM path is
separate from the official Slint runtime and renderer. They do not establish a
universal legal conclusion for every release, jurisdiction, or downstream
product. The actual package and artifacts, their origin and dependencies, the
applicable terms, and the product's public presentation remain decisive. This
overview provides a basis for that review but does not replace individual legal
advice.

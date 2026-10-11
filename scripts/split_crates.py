#!/usr/bin/env python3
"""Split src/plugins into per-plugin crates and move the kernel to lariv-core."""

from __future__ import annotations

import os
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"

PLUGINS = [
    "blog",
    "contacts",
    "crm",
    "customer",
    "dashboard",
    "documents",
    "export",
    "filesystem",
    "finance_accounts",
    "finance_common",
    "finance_creditnotes",
    "finance_customer",
    "finance_indian",
    "finance_invoices",
    "finance_purchases",
    "finance_products",
    "finance_taxes",
    "forms",
    "hr",
    "import",
    "llm_assistant",
    "meets",
    "otp",
    "pwa",
    "signing",
    "signup",
    "tasks",
    "users",
    "website",
]

# feature name -> (extra feature deps, plugin crate deps)
FEATURE_EDGES = {
    "plugin-users": ([], ["users"]),
    "plugin-dashboard": (["plugin-users"], ["dashboard"]),
    "plugin-blog": (["plugin-users"], ["blog"]),
    "plugin-forms": (["plugin-users", "plugin-filesystem"], ["forms"]),
    "plugin-filesystem": (["plugin-users"], ["filesystem"]),
    "plugin-website": (["plugin-filesystem", "plugin-users"], ["website"]),
    "plugin-llm-assistant": (["plugin-users", "plugin-filesystem", "cap-llm"], ["llm_assistant"]),
    "plugin-otp": (["plugin-users"], ["otp"]),
    "plugin-pwa": ([], ["pwa"]),
    "plugin-export": (["plugin-users"], ["export"]),
    "plugin-import": (["plugin-users"], ["import"]),
    "plugin-signup": (["plugin-users"], ["signup"]),
    "plugin-hr": (
        [
            "plugin-users",
            "plugin-forms",
            "plugin-website",
            "plugin-otp",
            "plugin-documents",
            "plugin-dashboard",
        ],
        ["hr"],
    ),
    "plugin-meets": (["plugin-users", "plugin-filesystem"], ["meets"]),
    "finance-common": ([], ["finance_common"]),
    "plugin-finance-accounts": (["plugin-users", "finance-common"], ["finance_accounts"]),
    "plugin-customer": (["plugin-users"], ["customer"]),
    "plugin-contacts": (["plugin-users"], ["contacts"]),
    "plugin-crm": (["plugin-contacts"], ["crm"]),
    "plugin-tasks": (["plugin-users"], ["tasks"]),
    "plugin-documents": (["plugin-filesystem"], ["documents"]),
    "plugin-signing": (
        ["plugin-users", "plugin-documents", "plugin-filesystem"],
        ["signing"],
    ),
    "plugin-finance-customer": (
        ["plugin-customer", "plugin-finance-accounts"],
        ["finance_customer"],
    ),
    "plugin-finance-taxes": (["plugin-finance-accounts"], ["finance_taxes"]),
    "plugin-finance-products": (
        ["plugin-finance-accounts", "plugin-finance-taxes"],
        ["finance_products"],
    ),
    "plugin-finance-creditnotes": (["plugin-finance-accounts"], ["finance_creditnotes"]),
    "plugin-finance-invoices": (
        [
            "plugin-finance-accounts",
            "plugin-finance-customer",
            "plugin-finance-products",
            "plugin-finance-taxes",
            "plugin-finance-creditnotes",
            "plugin-filesystem",
        ],
        ["finance_invoices"],
    ),
    "plugin-finance-purchases": (
        [
            "plugin-finance-accounts",
            "plugin-finance-customer",
            "plugin-finance-products",
            "plugin-finance-taxes",
            "plugin-filesystem",
        ],
        ["finance_purchases"],
    ),
    "plugin-finance-indian": (["plugin-finance-accounts"], ["finance_indian"]),
    "plugin-finance": (["plugin-finance-invoices", "plugin-finance-purchases", "plugin-finance-indian"], []),
}

PLUGIN_DEPS = {
    "blog": ["users"],
    "contacts": ["users"],
    "crm": ["contacts"],
    "customer": ["users"],
    "dashboard": ["users"],
    "documents": ["filesystem"],
    "export": ["users"],
    "filesystem": ["users"],
    "finance_accounts": ["users", "finance_common"],
    "finance_common": [],
    "finance_creditnotes": ["finance_accounts"],
    "finance_customer": ["customer", "finance_accounts"],
    "finance_indian": ["finance_accounts"],
    "finance_invoices": [
        "finance_accounts",
        "finance_customer",
        "finance_products",
        "finance_taxes",
        "finance_creditnotes",
        "filesystem",
    ],
    "finance_purchases": [
        "finance_accounts",
        "finance_customer",
        "finance_products",
        "finance_taxes",
        "filesystem",
    ],
    "finance_products": ["finance_accounts", "finance_taxes"],
    "finance_taxes": ["finance_accounts"],
    "forms": ["users", "filesystem"],
    "hr": ["users", "forms", "website", "otp", "documents", "dashboard"],
    "import": ["users"],
    "llm_assistant": ["users", "filesystem"],
    "meets": ["users", "filesystem"],
    "otp": ["users"],
    "pwa": [],
    "signing": ["users", "documents", "filesystem"],
    "signup": ["users"],
    "tasks": ["users"],
    "users": [],
    "website": ["filesystem", "users"],
}

CORE_FEATURES = {
    "filesystem": ["typst"],
    "llm_assistant": ["cap-llm", "typst"],
    "finance_invoices": ["typst"],
    "finance_purchases": ["typst"],
}

CORE_MODULES = {
    "app",
    "apps",
    "auth_hooks",
    "capability",
    "command",
    "components",
    "config",
    "datetime",
    "db",
    "docs",
    "duration",
    "export",
    "filestore",
    "genai",
    "grapesjs",
    "hooks",
    "html_form",
    "http",
    "layers",
    "length",
    "llm_tools",
    "migration",
    "picker",
    "plugin_install",
    "plugin_routes",
    "role_registry",
    "rt",
    "rune_env",
    "tag",
    "template",
    "traits",
    "typst",
    "views",
    "web",
    "paste",
}

MACROS = {
    "define_plugin_install",
    "define_passthrough_cap",
    "define_passthrough_cap_impl",
    "define_register_items",
    "define_replace_templates",
    "define_register_apps",
    "define_register_migrations",
    "define_register_export",
    "define_plugin_routes",
    "swap_key",
    "impl_picker_modal",
    "impl_create_modal",
    "main",
}

SELF = "__lariv_self__"


def plugin_crate(name: str) -> str:
    return f"lariv_plugin_{name}"


def plugin_package(name: str) -> str:
    return "lariv-plugin-" + name.replace("_", "-")


def rewrite(text: str, plugin: str) -> str:
    text = text.replace(f"crate::plugins::{plugin}::", f"crate::{SELF}::")
    for other in sorted(PLUGINS, key=len, reverse=True):
        if other == plugin:
            continue
        text = text.replace(f"crate::plugins::{other}::", f"{plugin_crate(other)}::")
        text = text.replace(f"crate::plugins::{other}", plugin_crate(other))
    text = text.replace("lariv_rs::", "lariv_core::")
    text = rewrite_use_groups(text, plugin)
    core_names = sorted(CORE_MODULES | MACROS, key=len, reverse=True)
    for name in core_names:
        text = re.sub(rf"(?<!\$)crate::{name}\b", f"lariv_core::{name}", text)
    text = text.replace(f"crate::{SELF}::", "crate::")
    text = text.replace(f"{SELF}::", "")
    return text


def rewrite_use_groups(text: str, plugin: str) -> str:
    pattern = re.compile(
        r"(?P<vis>pub(?:\([^)]*\))?\s+)?use\s+crate::\{",
    )
    out = []
    idx = 0
    for match in pattern.finditer(text):
        start = match.start()
        brace = match.end() - 1
        end = find_matching(text, brace)
        if end < 0:
            continue
        body = text[brace + 1 : end]
        out.append(text[idx:start])
        out.append(render_use_group(match.group("vis") or "", body, plugin))
        # include trailing semicolon
        semi = end + 1
        if semi < len(text) and text[semi] == ";":
            semi += 1
        idx = semi
    out.append(text[idx:])
    return "".join(out)


def find_matching(text: str, open_idx: int) -> int:
    depth = 0
    for i in range(open_idx, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
    return -1


def split_top(body: str) -> list[str]:
    parts = []
    depth = 0
    buf = []
    for ch in body:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(buf).strip())
            buf = []
            continue
        buf.append(ch)
    tail = "".join(buf).strip()
    if tail:
        parts.append(tail)
    return [p for p in parts if p]


def render_use_group(vis: str, body: str, plugin: str) -> str:
    vis = vis or ""
    lines = []
    for part in split_top(body):
        part = re.sub(rf"plugins::{plugin}::", f"{SELF}::", part)
        for other in sorted(PLUGINS, key=len, reverse=True):
            if other == plugin:
                continue
            part = part.replace(f"plugins::{other}::", f"{plugin_crate(other)}::")
        first = part.split("::", 1)[0]
        if part.startswith(f"{SELF}::"):
            rest = part[len(SELF) + 2 :]
            lines.append(f"{vis}use crate::{rest};")
        elif first in CORE_MODULES or first in MACROS or first.startswith("lariv_"):
            if first.startswith("lariv_"):
                lines.append(f"{vis}use {part};")
            else:
                lines.append(f"{vis}use lariv_core::{part};")
        else:
            lines.append(f"{vis}use crate::{part};")
    return "\n".join(lines)


def rust_idents(text: str) -> set[str]:
    return set(re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)::", text))


def move_tree(src: Path, dst: Path) -> None:
    dst.parent.mkdir(parents=True, exist_ok=True)
    if src.is_dir():
        shutil.copytree(src, dst)
        shutil.rmtree(src)
    else:
        shutil.copy2(src, dst)
        src.unlink()


def main() -> None:
    core_src = ROOT / "lariv-core" / "src"
    core_src.mkdir(parents=True, exist_ok=True)

    smoke = SRC / "components" / "smoke_tests.rs"
    if smoke.exists():
        text = smoke.read_text()
        text = text.replace("crate::", "lariv_rs::")
        text = re.sub(r"^#!\[cfg\(test\)\]\s*", "", text)
        if text.startswith("mod tests {"):
            text = text[len("mod tests {") :]
            text = text.rstrip()
            if text.endswith("}"):
                text = text[:-1]
        (ROOT / "tests" / "component_smoke.rs").write_text(
            "#![cfg(all(feature = \"plugin-dashboard\", feature = \"plugin-users\"))]\n" + text
        )
        smoke.unlink()

    for child in list(SRC.iterdir()):
        if child.name in {"plugins", "bin", "plugins.rs", "lib.rs"}:
            continue
        move_tree(child, core_src / child.name)

    lib = (SRC / "lib.rs").read_text()
    core_lib = lib.replace("pub mod plugins;\n", "")
    (core_src / "lib.rs").write_text(core_lib)
    (SRC / "lib.rs").unlink()

    plugins_dir = SRC / "plugins"
    for name in PLUGINS:
        dest = ROOT / "plugins" / name / "src"
        dest.mkdir(parents=True, exist_ok=True)
        rs = plugins_dir / f"{name}.rs"
        folder = plugins_dir / name
        text = rewrite(rs.read_text(), name)
        (dest / "lib.rs").write_text(text)
        rs.unlink()
        if folder.exists():
            for path in folder.rglob("*"):
                if path.is_dir():
                    continue
                rel = path.relative_to(folder)
                target = dest / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                if path.suffix == ".rs":
                    target.write_text(rewrite(path.read_text(), name))
                else:
                    shutil.copy2(path, target)
            shutil.rmtree(folder)

    if plugins_dir.exists():
        shutil.rmtree(plugins_dir)
    plugins_rs = SRC / "plugins.rs"
    if plugins_rs.exists():
        plugins_rs.unlink()

    write_facade_lib()
    write_cargo()
    print("split complete")


def write_facade_lib() -> None:
    lines = [
        "#![feature(impl_trait_in_assoc_type)]",
        "#![recursion_limit = \"512\"]",
        "",
        "//! Lariv application kernel and bundled plugins.",
        "//!",
        "//! This crate re-exports [`lariv_core`] and each plugin behind the same feature flags",
        "//! and module paths as before the workspace split.",
        "extern crate self as lariv_rs;",
        "",
        "pub use lariv_core::{",
        "    app, apps, auth_hooks, capability, command, components, config, datetime, db, docs,",
        "    duration, export, filestore, genai, grapesjs, hooks, html_form, http, layers, length,",
        "    migration, picker, plugin_install, plugin_routes, role_registry, rt, tag, template,",
        "    traits, views, web,",
        "};",
        "#[cfg(feature = \"cap-llm\")]",
        "pub use lariv_core::llm_tools;",
        "#[cfg(not(feature = \"cap-llm\"))]",
        "pub use lariv_core::llm_tools;",
        "#[cfg(feature = \"cap-llm\")]",
        "pub use lariv_core::rune_env;",
        "#[cfg(not(feature = \"cap-llm\"))]",
        "pub use lariv_core::rune_env;",
        "#[cfg(feature = \"typst\")]",
        "pub use lariv_core::typst;",
        "pub use lariv_core::{",
        "    define_passthrough_cap, define_plugin_install, define_register_apps, define_register_export,",
        "    define_register_items, define_register_migrations, define_replace_templates, impl_create_modal,",
        "    impl_picker_modal, swap_key,",
        "};",
        "pub use lariv_rs_macros::{define_plugin_routes, main};",
        "pub use paste;",
        "",
        "pub mod plugins {",
    ]
    feature_for = {
        "finance_common": "finance-common",
    }
    for name in PLUGINS:
        feature = feature_for.get(name, "plugin-" + name.replace("_", "-"))
        lines.append(f'    #[cfg(feature = "{feature}")]')
        lines.append(f"    pub use {plugin_crate(name)} as {name};")
    lines.append("}")
    lines.append("")
    (SRC / "lib.rs").write_text("\n".join(lines) + "\n")


def write_cargo() -> None:
    original = (ROOT / "Cargo.toml").read_text()
    # Keep header through workspace.dependencies macros line, then rebuild.
    header = original.split("[package]", 1)[0]
    pkg = re.search(r"\[package\][\s\S]*?(?=\n\[features\])", original).group(0)
    dev = re.search(r"\[dev-dependencies\][\s\S]*?(?=\n\[\[test\]\])", original).group(0)
    tests = re.search(r"\[\[test\]\][\s\S]*?(?=\n\[workspace\.lints)", original).group(0)
    lints = original.split("[workspace.lints.clippy]", 1)[1]
    # dependencies block
    deps_block = re.search(
        r"\[dependencies\]\n([\s\S]*?)\n\[dev-dependencies\]", original
    ).group(1)
    workspace_deps = deps_to_workspace(deps_block)
    if "lariv-core" not in header:
        header = header.rstrip() + "\n"
        header = header.replace(
            'members = [".", "lariv-rs-macros"]',
            'members = [".", "lariv-rs-macros", "lariv-core", "plugins/*"]',
        )
        # insert workspace deps before [package] — header already ends at [package]
        # header includes [workspace.dependencies] macros only
        if "[workspace.dependencies]" in header:
            header = header.replace(
                "[workspace.dependencies]\nlariv-rs-macros = { version = \"0.1.0\", path = \"lariv-rs-macros\" }\n",
                "[workspace.dependencies]\nlariv-rs-macros = { version = \"0.1.0\", path = \"lariv-rs-macros\" }\n"
                "lariv-core = { version = \"0.1.0\", path = \"lariv-core\" }\n"
                + "\n".join(f"{k} = {v}" for k, v in workspace_deps)
                + "\n",
            )

    features = facade_features()
    facade_deps = facade_dependencies()
    cargo = (
        header
        + "\n"
        + pkg
        + "\n"
        + features
        + "\n"
        + facade_deps
        + "\n"
        + dev
        + "\n"
        + tests
        + "\n[workspace.lints.clippy]"
        + lints
    )
    # component smoke test
    if 'name = "component_smoke"' not in cargo:
        cargo = cargo.replace(
            "[[test]]\nname = \"genai_files_latency\"",
            '[[test]]\nname = "component_smoke"\npath = "tests/component_smoke.rs"\n'
            'required-features = ["plugin-dashboard", "plugin-users"]\n\n'
            "[[test]]\nname = \"genai_files_latency\"",
        )
    (ROOT / "Cargo.toml").write_text(cargo)
    write_core_cargo(workspace_deps)
    for name in PLUGINS:
        write_plugin_cargo(name)


def deps_to_workspace(block: str) -> list[tuple[str, str]]:
    entries = []
    current = None
    buf = []
    depth = 0
    for line in block.splitlines():
        if current is None:
            m = re.match(r"^([A-Za-z0-9_-]+)\s*=\s*(.*)$", line)
            if not m:
                continue
            current = m.group(1)
            rest = m.group(2)
            buf = [rest]
            depth = rest.count("{") - rest.count("}")
            if depth <= 0 and not rest.endswith("\\"):
                entries.append((current, strip_optional(" ".join(buf))))
                current = None
                buf = []
            continue
        buf.append(line.strip())
        depth += line.count("{") - line.count("}")
        if depth <= 0:
            entries.append((current, strip_optional(" ".join(buf))))
            current = None
            buf = []
    return entries


def strip_optional(expr: str) -> str:
    expr = re.sub(r",?\s*optional\s*=\s*true", "", expr)
    expr = re.sub(r"\{\s*,", "{", expr)
    expr = re.sub(r",\s*\}", " }", expr)
    return expr.strip()


def facade_features() -> str:
    lines = ["[features]", 'default = ["full"]']
    full = [
        "plugin-users",
        "plugin-dashboard",
        "plugin-blog",
        "plugin-forms",
        "plugin-filesystem",
        "plugin-website",
        "plugin-llm-assistant",
        "plugin-otp",
        "plugin-pwa",
        "plugin-export",
        "plugin-import",
        "plugin-signup",
        "plugin-hr",
        "plugin-meets",
    ]
    lines.append("full = [")
    for item in full:
        lines.append(f'    "{item}",')
    lines.append("]")
    lines.append('cap-llm = ["lariv-core/cap-llm"]')
    lines.append('typst = ["lariv-core/typst"]')
    weak = {
        "plugin-llm-assistant": [
            "lariv-plugin-finance-invoices?/plugin-llm-assistant",
            "lariv-plugin-finance-products?/plugin-llm-assistant",
            "lariv-plugin-finance-customer?/plugin-llm-assistant",
            "lariv-plugin-documents?/plugin-llm-assistant",
        ],
        "plugin-export": ["lariv-plugin-import?/plugin-export"],
        "plugin-finance-customer": ["lariv-plugin-customer/plugin-finance-customer"],
    }
    for feature, (parents, plugins) in FEATURE_EDGES.items():
        parts = list(parents)
        if feature == "plugin-users":
            parts.append("lariv-core/plugin-users")
        if feature in ("plugin-filesystem", "plugin-llm-assistant", "plugin-finance-invoices"):
            parts.append("typst")
        if feature == "plugin-llm-assistant":
            parts.append("cap-llm")
        for plugin in plugins:
            parts.append("dep:" + plugin_package(plugin))
        parts.extend(weak.get(feature, []))
        rendered = ", ".join(f'"{p}"' for p in parts)
        lines.append(f"{feature} = [{rendered}]")
    return "\n".join(lines) + "\n"


def facade_dependencies() -> str:
    lines = [
        "[dependencies]",
        "lariv-core = { workspace = true }",
        "lariv-rs-macros.workspace = true",
        "paste = { workspace = true }",
        "tokio = { workspace = true }",
        "anyhow = { workspace = true }",
        "tracing-subscriber = { workspace = true }",
    ]
    for name in PLUGINS:
        lines.append(
            f'{plugin_package(name)} = {{ path = "plugins/{name}", optional = true }}'
        )
    return "\n".join(lines) + "\n"


def write_core_cargo(workspace_deps: list[tuple[str, str]]) -> None:
    core_text = "\n".join((ROOT / "lariv-core" / "src").rglob("*.rs") and [])
    texts = []
    for path in (ROOT / "lariv-core" / "src").rglob("*.rs"):
        texts.append(path.read_text())
    idents = rust_idents("\n".join(texts))
    dep_names = {cargo_to_rust(name): name for name, _ in workspace_deps}
    # always
    needed = ["lariv-rs-macros", "paste", "tokio", "axum", "serde", "thiserror", "tracing"]
    for rust_name, cargo_name in dep_names.items():
        if rust_name in idents or cargo_name in needed:
            needed.append(cargo_name)
    needed = list(dict.fromkeys(needed))
    lines = [
        "[package]",
        'name = "lariv-core"',
        'version = "0.1.0"',
        "edition.workspace = true",
        "license.workspace = true",
        "authors.workspace = true",
        "repository.workspace = true",
        'description = "Lariv application kernel"',
        "",
        "[features]",
        'plugin-users = ["dep:phonenumber"]',
        'cap-llm = ["dep:reqwest", "dep:rune"]',
        'typst = ["dep:typst", "dep:typst-pdf", "dep:typst-kit", "dep:typst-syntax", "dep:comemo"]',
        "",
        "[dependencies]",
        "lariv-rs-macros.workspace = true",
    ]
    optional = {"phonenumber", "reqwest", "rune", "typst", "typst-pdf", "typst-kit", "typst-syntax", "comemo"}
    for cargo_name in needed:
        if cargo_name == "lariv-rs-macros":
            continue
        if cargo_name in optional:
            lines.append(f"{cargo_name} = {{ workspace = true, optional = true }}")
        else:
            lines.append(f"{cargo_name} = {{ workspace = true }}")
    lines.append("")
    lines.append("[lints]")
    lines.append("workspace = true")
    lines.append("")
    (ROOT / "lariv-core" / "Cargo.toml").write_text("\n".join(lines))


def cargo_to_rust(name: str) -> str:
    return name.replace("-", "_")


def write_plugin_cargo(name: str) -> None:
    src = ROOT / "plugins" / name / "src"
    text = "\n".join(p.read_text() for p in src.rglob("*.rs"))
    idents = rust_idents(text)
    # workspace dep rust names discovered later; use a static map filled from root after write.
    # Read workspace deps from the cargo file we just wrote? write_cargo calls this after write.
    root_toml = (ROOT / "Cargo.toml").read_text()
    ws = re.search(r"\[workspace\.dependencies\]\n([\s\S]*?)\n\[package\]", root_toml)
    block = ws.group(1) if ws else ""
    cargo_names = re.findall(r"^([A-Za-z0-9_-]+)\s*=", block, re.M)
    dep_names = {cargo_to_rust(n): n for n in cargo_names}
    lines = [
        "[package]",
        f'name = "{plugin_package(name)}"',
        'version = "0.1.0"',
        "edition.workspace = true",
        "license.workspace = true",
        "authors.workspace = true",
        "repository.workspace = true",
        f'description = "Lariv {name} plugin"',
        "",
        "[features]",
    ]
    if name == "customer":
        lines.append("plugin-finance-customer = []")
    if name in {"finance_invoices", "finance_purchases", "finance_products", "finance_customer", "documents"}:
        extra = ""
        if name == "documents":
            extra = ', "lariv_core/cap-llm"'
        lines.append(
            f'plugin-llm-assistant = ["dep:lariv-plugin-llm-assistant"{extra}]'
        )
    if name == "import":
        lines.append('plugin-export = ["dep:lariv-plugin-export"]')
    if lines[-1] == "[features]":
        lines.append("default = []")
    features = CORE_FEATURES.get(name, [])
    feat = ""
    if features:
        feat = ", features = [" + ", ".join(f'"{f}"' for f in features) + "]"
    lines += [
        "",
        "[dependencies]",
        f'lariv_core = {{ package = "lariv-core", path = "../../lariv-core"{feat} }}',
        "lariv-rs-macros = { workspace = true }",
    ]
    for other in PLUGIN_DEPS.get(name, []):
        lines.append(
            f'{plugin_package(other)} = {{ path = "../{other}" }}'
        )
    if name in {"finance_invoices", "finance_purchases", "finance_products", "finance_customer", "documents"}:
        lines.append(
            'lariv-plugin-llm-assistant = { path = "../llm_assistant", optional = true }'
        )
    if name == "import":
        lines.append('lariv-plugin-export = { path = "../export", optional = true }')
    skip = {"lariv_core", "lariv_rs", "lariv_rs_macros", "std", "core", "alloc"}
    skip |= {plugin_crate(p) for p in PLUGINS}
    for rust_name, cargo_name in sorted(dep_names.items()):
        if rust_name in skip:
            continue
        if rust_name in idents:
            lines.append(f"{cargo_name} = {{ workspace = true }}")
    lines += ["", "[lints]", "workspace = true", ""]
    (ROOT / "plugins" / name / "Cargo.toml").write_text("\n".join(lines))


if __name__ == "__main__":
    main()

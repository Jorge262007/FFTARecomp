#!/usr/bin/env python3
"""Create and validate one-file menu translations using only the standard library."""
import argparse
import json
from pathlib import Path
import re
import unicodedata

DIRECTORY = Path(__file__).resolve().parents[1] / "crates/input-config/languages"
CODE = re.compile(r"[a-z]{2,3}(?:-[a-z0-9]{2,8})*\Z", re.ASCII)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key {key!r}")
        result[key] = value
    return result


def placeholders(text):
    names = set()
    while match := re.search(r"[{}]", text):
        if match[0] == "}":
            raise ValueError("unmatched closing brace")
        text = text[match.start() + 1:]
        end = text.find("}")
        if end == -1:
            raise ValueError("unmatched opening brace")
        name = text[:end]
        if not re.fullmatch(r"[A-Za-z0-9_]+", name, re.ASCII):
            raise ValueError(f"invalid placeholder {{{name}}}")
        names.add(name)
        text = text[end + 1:]
    return names


def valid_name(name):
    return isinstance(name, str) and name.strip() and not any(unicodedata.category(c) == "Cc" for c in name)


def read_catalog(path):
    try:
        data = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
        if not isinstance(data, dict) or set(data) != {"code", "name", "messages"}:
            raise ValueError("expected exactly code, name and messages")
        if not isinstance(data["code"], str) or not CODE.fullmatch(data["code"]):
            raise ValueError("code must be lowercase, e.g. fr or pt-br")
        if path.stem != data["code"]:
            raise ValueError("filename must match code")
        if not valid_name(data["name"]):
            raise ValueError("name must be nonempty without control characters")
        if not isinstance(data["messages"], dict):
            raise ValueError("messages must be an object")
        for key, value in data["messages"].items():
            if not key.strip() or not isinstance(value, str) or not value.strip():
                raise ValueError(f"empty key or invalid translation: {key!r}")
            try:
                placeholders(value)
            except ValueError as error:
                raise ValueError(f"message {key!r}: {error}") from error
        return data
    except (ValueError, OSError) as error:
        raise ValueError(f"{path}: {error}") from error


def check(directory):
    catalogs = sorted((read_catalog(path) for path in directory.glob("*.json")), key=lambda c: (c["code"] != "en", c["code"]))
    if not catalogs or catalogs[0]["code"] != "en" or not catalogs[0]["messages"]:
        raise ValueError("en.json with base messages is required")
    english = catalogs[0]["messages"]
    names = set()
    for catalog in catalogs:
        if catalog["name"] in names:
            raise ValueError(f"{catalog['code']}.json: duplicate display name {catalog['name']!r}")
        names.add(catalog["name"])
        for key, value in catalog["messages"].items():
            if key not in english:
                raise ValueError(f"{catalog['code']}.json: unknown message key {key!r}")
            if placeholders(value) != placeholders(english[key]):
                raise ValueError(f"{catalog['code']}.json: placeholders differ from English for {key!r}")
    return catalogs


def create(directory, code, name):
    if not CODE.fullmatch(code) or not valid_name(name):
        raise ValueError("use a lowercase language code (fr, pt-br) and a nonempty display name")
    english = read_catalog(directory / "en.json")
    if any(catalog["name"] == name for catalog in check(directory)):
        raise ValueError(f"display name already used: {name!r}")
    path = directory / f"{code}.json"
    with path.open("x", encoding="utf-8") as output:
        json.dump({"code": code, "name": name, "messages": english["messages"]}, output, ensure_ascii=False, indent=2)
        output.write("\n")
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=DIRECTORY, help=argparse.SUPPRESS)
    sub = parser.add_subparsers(dest="command", required=True)
    add = sub.add_parser("create", help="Copy English into a new language file; translate its values")
    add.add_argument("code", help="Language code, for example fr or pt-br")
    add.add_argument("--name", required=True, help="Native language name, for example Français")
    sub.add_parser("check", help="Validate every file, message key and placeholder")
    args = parser.parse_args()
    try:
        if args.command == "create":
            path = create(args.directory, args.code, args.name)
            print(f"Created {path}. Translate message values, run check, then rebuild; registration is automatic.")
        else:
            catalogs = check(args.directory)
            total = len(catalogs[0]["messages"])
            for catalog in catalogs:
                missing = total - len(catalog["messages"])
                print(f"{catalog['code']}.json: {catalog['name']} — {len(catalog['messages'])}/{total} entries; {missing} use English fallback")
            print("Menu languages: OK")
    except (ValueError, OSError) as error:
        parser.exit(1, f"Menu languages: {error}\n")


if __name__ == "__main__":
    main()

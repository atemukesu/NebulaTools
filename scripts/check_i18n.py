import os
import re
import json
import sys


def find_tr_keys(directory):
    keys = set()
    # Regex to match .tr("key") or .tr( "key" ) even across newlines
    # Supports: .tr("key"), .tr( "key" ), .i18n.tr("key")
    pattern = re.compile(r'\.tr\s*\(\s*["\']([^"\']+)["\']\s*\)', re.MULTILINE)

    for root, dirs, files in os.walk(directory):
        for file in files:
            if file.endswith(".rs"):
                path = os.path.join(root, file)
                try:
                    with open(path, "r", encoding="utf-8") as f:
                        content = f.read()
                        matches = pattern.findall(content)
                        for m in matches:
                            keys.add(m)
                except Exception as e:
                    print(f"Error reading {path}: {e}")
    return keys


def check_langs(src_dir, lang_dir):
    used_keys = find_tr_keys(src_dir)
    print(f"--- [Found {len(used_keys)} unique keys in Rust source code] ---")

    lang_files = [f for f in os.listdir(lang_dir) if f.endswith(".json")]

    for lang_file in lang_files:
        path = os.path.join(lang_dir, lang_file)
        print(f"\nChecking: {lang_file}")
        try:
            with open(path, "r", encoding="utf-8") as f:
                lang_data = json.load(f)
                defined_keys = set(lang_data.keys())

                missing = used_keys - defined_keys
                extra = defined_keys - used_keys

                if missing:
                    print("  ❌ Missing translations (Used in code but NOT in JSON):")
                    for k in sorted(missing):
                        print(f"    - {k}")
                else:
                    print("  ✅ No missing keys.")

                if extra:
                    print("  ⚠️ Orphan keys (In JSON but NOT used in code):")
                    for k in sorted(extra):
                        print(f"    - {k}")
                else:
                    print("  ✅ No orphan keys.")
        except Exception as e:
            print(f"Error checking {lang_file}: {e}")


if __name__ == "__main__":
    # Force UTF-8 output for Windows consoles
    if sys.stdout.encoding.lower() != "utf-8":
        import io

        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")

    # Assuming script is run from project root
    src = os.path.join(os.getcwd(), "src")
    langs = os.path.join(os.getcwd(), "assets", "lang")

    if not os.path.exists(src) or not os.path.exists(langs):
        print(
            "Error: Could not find 'src' or 'assets/lang' directory from current path."
        )
        sys.exit(1)

    check_langs(src, langs)

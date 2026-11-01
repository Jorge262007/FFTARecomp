import json
from pathlib import Path
import tempfile
import unittest
import menu_language as language


class LanguageFiles(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ffta-language-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.english = {"code": "en", "name": "English", "messages": {"Hello": "Hello", "Saved {slot}": "Saved {slot}"}}
        self.write("en", self.english)

    def write(self, code, content):
        (self.root / f"{code}.json").write_text(json.dumps(content), encoding="utf-8")

    def test_create_adds_one_complete_file_and_preserves_existing_files(self):
        before = (self.root / "en.json").read_bytes()
        path = language.create(self.root, "fr", "Français")
        self.assertEqual(path.name, "fr.json")
        catalog = json.loads(path.read_text())
        self.assertEqual(catalog["messages"], self.english["messages"])
        self.assertEqual((self.root / "en.json").read_bytes(), before)
        with self.assertRaises(FileExistsError):
            language.create(self.root, "fr", "French")
        self.assertEqual(path.read_text(), json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
        self.assertEqual([c["code"] for c in language.check(self.root)], ["en", "fr"])

    def test_partial_translation_uses_base_keys_and_placeholder_check(self):
        self.write("it", {"code": "it", "name": "Italiano", "messages": {"Hello": "Ciao"}})
        self.assertEqual(len(language.check(self.root)[1]["messages"]), 1)
        self.write("it", {"code": "it", "name": "Italiano", "messages": {"Saved {slot}": "Salvato {other}"}})
        with self.assertRaisesRegex(ValueError, "placeholders"):
            language.check(self.root)

    def test_duplicate_keys_bad_metadata_unknown_messages_and_traversal_refuse(self):
        for content in [
            '{"code":"it","name":"Italiano","messages":{"Hello":"Uno","Hello":"Due"}}',
            '{"code":"fr","name":"Italiano","messages":{}}',
            '{"code":"it","name":"Italiano","messages":{"Unknown":"Sconosciuto"}}',
            '{"code":"it","name":"English","messages":{}}',
            '{"code":"it","name":"Italiano","messages":{"Hello":"Bad {"}}',
        ]:
            (self.root / "it.json").write_text(content)
            with self.assertRaises(ValueError):
                language.check(self.root)
        for code in ["../fr", "FR", "", "en-", "english"]:
            with self.assertRaises(ValueError):
                language.create(self.root, code, "French")


if __name__ == "__main__":
    unittest.main()

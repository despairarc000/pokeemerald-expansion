"""Regression checks for numeric layouts without running the full ROM build."""
import json
import struct
import tempfile
import unittest
from pathlib import Path
import importlib.util

ENGINE = Path(__file__).resolve().parents[3] / "tools/textmap/build_textmaps.py"
spec = importlib.util.spec_from_file_location("build_textmaps", ENGINE)
tool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tool)

class NumericLayoutTests(unittest.TestCase):
    def test_exact_packed_native_cell_bits(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "layout.mapstudio.json"
            path.write_text(json.dumps({
                "format": "mapstudio-numeric-v1",
                "header": {
                    "id":"LAYOUT_MEADOWMERE_MAPA",
                    "name":"Meadowmere_MapA_Layout",
                    "primary":"gTileset_General",
                    "secondary":"gTileset_Petalburg",
                    "group":"gMapGroup_Meadowmere"},
                "border":[0x170]*4,
                "cells":[[0x3555,0x3170],[0x3001,0x300D]]
            }),encoding="utf-8")
            h,b,rows = tool.parse_numeric_layout(str(path),"Meadowmere_MapA")
            self.assertEqual(b,[0x170]*4)
            self.assertEqual(tool.pack_numeric_cells(rows),struct.pack("<4H",0x3555,0x3170,0x3001,0x300D))
            self.assertEqual(h["name"],"Meadowmere_MapA_Layout")

    def test_malformed_packed_cells_are_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/"layout.mapstudio.json"
            path.write_text(json.dumps({
                "format":"mapstudio-numeric-v1",
                "header":{k:"X" for k in ("id","name","primary","secondary","group")},
                "border":[0]*4,
                "cells":[[1,2],[3,65536]]
            }),encoding="utf-8")
            with self.assertRaises(SystemExit):
                tool.parse_numeric_layout(str(path),"M")

if __name__=="__main__":
    unittest.main()

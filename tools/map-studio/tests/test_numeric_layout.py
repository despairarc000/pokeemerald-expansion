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


class CompilerIntegrationTests(unittest.TestCase):
    def test_full_generator_writes_native_bins_and_registrations(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)
            src=root/"custom_maps"/"Meadowmere_Test"
            src.mkdir(parents=True)
            (src/"layout.mapstudio.json").write_text(json.dumps({
                "format":"mapstudio-numeric-v1",
                "header":{
                    "id":"LAYOUT_MEADOWMERE_TEST",
                    "name":"Meadowmere_Test_Layout",
                    "primary":"gTileset_General",
                    "secondary":"gTileset_Petalburg",
                    "group":"gMapGroup_Meadowmere"
                },
                "border":[0x170]*4,
                "cells":[[0x3001,0x3170],[0x300d,0x3555]]
            }),encoding="utf-8")
            source_map={
                "id":"MAP_MEADOWMERE_TEST",
                "name":"Meadowmere_Test",
                "layout":"LAYOUT_MEADOWMERE_TEST",
                "object_events":[],
                "warp_events":[],
                "coord_events":[],
                "bg_events":[],
                "connections":[]
            }
            (src/"map.json").write_text(json.dumps(source_map),encoding="utf-8")
            layouts=root/"data"/"layouts"/"layouts.json"
            groups=root/"data"/"maps"/"map_groups.json"
            scripts=root/"data"/"event_scripts.s"
            layouts.parent.mkdir(parents=True)
            groups.parent.mkdir(parents=True)
            layouts.write_text(json.dumps({"layouts":[]}),encoding="utf-8")
            groups.write_text(json.dumps({"group_order":[]}),encoding="utf-8")
            scripts.write_text('\t.include "data/scripts/shared_secret_base.inc"\n',encoding="utf-8")

            with patch.multiple(tool,ROOT=str(root),CUSTOM=str(root/"custom_maps"),
                    LAYOUTS_JSON=str(layouts),GROUPS_JSON=str(groups),
                    EVENT_SCRIPTS=str(scripts)):
                tool.main()
                native=(root/"data"/"layouts"/"Meadowmere_Test"/"map.bin")
                expected=struct.pack("<4H",0x3001,0x3170,0x300d,0x3555)
                self.assertEqual(native.read_bytes(),expected)
                self.assertEqual((root/"data"/"layouts"/"Meadowmere_Test"/"border.bin").read_bytes(),
                    struct.pack("<4H",*[0x170]*4))
                self.assertEqual(json.loads(layouts.read_text())["layouts"][0]["width"],2)
                self.assertIn("Meadowmere_Test",json.loads(groups.read_text())["gMapGroup_Meadowmere"])
                self.assertIn('data/maps/Meadowmere_Test/scripts.inc',scripts.read_text())
                self.assertEqual(json.loads(
                    (root/"data"/"maps"/"Meadowmere_Test"/"map.json").read_text()),source_map)
                original_mtime=native.stat().st_mtime_ns
                tool.main()
                self.assertEqual(native.stat().st_mtime_ns,original_mtime)

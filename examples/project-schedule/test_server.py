"""Integration tests using the actual built RSPDL CLI."""
import json
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from http.server import HTTPServer

from server import Application, BackendError, handler_for


class ServerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.data = Path(self.directory.name) / 'saved.json'
        self.app = Application(data_file=self.data)

    def tearDown(self):
        self.app.close()
        self.directory.cleanup()

    def test_save_rechecks_and_rejection_preserves_saved_data(self):
        good = {'start_date': '2026-10-05', 'end_date': '2026-10-06'}
        self.assertTrue(self.app.check(good, save=True)['saved'])
        original = self.data.read_bytes()
        for bad in [
            {'start_date': '2026-10-06', 'end_date': '2026-10-05'},
            {'start_date': '2026-10-05', 'end_date': '2026-10-05'},
            {'start_date': '2026-02-30', 'end_date': '2026-10-05'},
            {'end_date': '2026-10-05'},
            {'start_date': None, 'end_date': '2026-10-05'},
            {**good, 'unexpected': 'field'},
        ]:
            with self.subTest(record=bad):
                result = self.app.check(bad, save=True)
                self.assertFalse(result['accepted'])
                self.assertFalse(result['saved'])
                self.assertEqual(original, self.data.read_bytes())
        with self.assertRaises(ValueError):
            self.app.check({**good, '$id': 'override'}, save=True)
        self.assertEqual(json.loads(original)['records']['project_schedule.project'][0]['$id'], 'project-1')

    def test_inclusive_alternate_source_and_snapshot(self):
        source = Path(self.directory.name) / 'inclusive.rspdl'
        source.write_text(self.app.source.replace('시작일보다 커야 한다.', '시작일보다 크거나 같아야 한다.'), encoding='utf-8')
        app = Application(source=source, data_file=self.data)
        try:
            source.write_text('invalid', encoding='utf-8')
            result = app.check({'start_date': '2026-10-05', 'end_date': '2026-10-05'}, save=True)
            self.assertTrue(result['saved'])
            self.assertEqual(app.schema()['source'], app.snapshot.read_text(encoding='utf-8'))
        finally:
            app.close()

    def test_compilation_error_and_timeout_fail_closed(self):
        source = Path(self.directory.name) / 'invalid.rspdl'
        source.write_text('잘못된 문장', encoding='utf-8')
        app = Application(source=source, data_file=self.data)
        try:
            self.assertFalse(app.check({}, save=True)['accepted'])
        finally:
            app.close()
        with patch('server.subprocess.run', side_effect=subprocess.TimeoutExpired('rspdl', 5)):
            with self.assertRaises(BackendError):
                self.app.check({}, save=True)
        self.assertFalse(self.data.exists())
        with patch('server.subprocess.run', return_value=subprocess.CompletedProcess([], 0, '{}', '')):
            self.assertFalse(self.app.check({}, save=True)['accepted'])
        self.assertFalse(self.data.exists())

    def test_unsupported_schema_cannot_accept(self):
        source = Path(self.directory.name) / 'unsupported.rspdl'
        source.write_text('@모듈 예시(example)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n', encoding='utf-8')
        app = Application(source=source, data_file=self.data)
        try:
            self.assertFalse(app.check({'value': 1}, save=True)['accepted'])
            self.assertFalse(self.data.exists())
        finally:
            app.close()

    def test_unsupported_date_rule_projections_cannot_save(self):
        original_rule = '프로젝트의 마감일은 시작일보다 커야 한다.'
        for replacement in [
            '',
            '프로젝트의 마감일과 시작일은 같아야 한다.',
            '프로젝트의 마감일은 "2026-10-05" 이상이어야 한다.',
            '프로젝트의 시작일은 시작일보다 크거나 같아야 한다.',
        ]:
            with self.subTest(rule=replacement):
                source = Path(self.directory.name) / 'unsupported-dates.rspdl'
                source.write_text(self.app.source.replace(original_rule, replacement), encoding='utf-8')
                app = Application(source=source, data_file=self.data)
                try:
                    self.assertEqual(app.compile_code, 0)
                    self.assertFalse(app.check({'start_date': '2026-10-05', 'end_date': '2026-10-05'}, save=True)['accepted'])
                    self.assertFalse(self.data.exists())
                finally:
                    app.close()

    def test_http_direct_save_and_routes(self):
        server = HTTPServer(('127.0.0.1', 0), handler_for(self.app))
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f'http://127.0.0.1:{server.server_port}'
        try:
            with urlopen(base + '/api/schema') as response:
                self.assertEqual(json.load(response), self.app.schema())
            def post(record):
                request = Request(base + '/api/save', json.dumps({'record': record}).encode(), {'Content-Type': 'application/json'})
                with urlopen(request) as response:
                    return json.load(response)
            self.assertFalse(post({'start_date': '2026-10-06', 'end_date': '2026-10-05'})['saved'])
            self.assertFalse(self.data.exists())
            self.assertTrue(post({'start_date': '2026-10-05', 'end_date': '2026-10-06'})['saved'])
            with self.assertRaises(HTTPError) as error:
                urlopen(base + '/../server.py')
            self.assertEqual(error.exception.code, 404)
            with self.assertRaises(HTTPError) as error:
                post({'$id': 'bad'})
            self.assertEqual(error.exception.code, 400)
        finally:
            server.shutdown()
            thread.join()
            server.server_close()


if __name__ == '__main__':
    unittest.main()

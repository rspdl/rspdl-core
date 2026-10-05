#!/usr/bin/env python3
"""Loopback-only local demo. RSPDL CLI owns all schema and value semantics."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
from http.server import BaseHTTPRequestHandler, HTTPServer

HERE = Path(__file__).resolve().parent
MAX_BODY = 64 * 1024


class BackendError(Exception):
    pass


class Application:
    def __init__(self, source=HERE / 'project.rspdl', rspdl=HERE.parent.parent / 'target/debug/rspdl', data_file=HERE / 'saved-project.json', timeout=5):
        self.source = Path(source).read_bytes().decode('utf-8')
        self.rspdl = str(Path(os.environ.get('RSPDL_BIN', str(rspdl))).resolve())
        self.data_file = Path(data_file).resolve()
        self.timeout = timeout
        self._directory = tempfile.TemporaryDirectory(prefix='rspdl-project-')
        self.snapshot = Path(self._directory.name) / 'project.rspdl'
        self.snapshot.write_text(self.source, encoding='utf-8')
        self.compile_code, self.compilation = self.invoke('compile')

    def close(self):
        self._directory.cleanup()

    def invoke(self, command, data=None):
        args = [self.rspdl, command, str(self.snapshot)]
        if data is not None:
            args += ['--data', str(data)]
        args += ['--json']
        try:
            result = subprocess.run(args, capture_output=True, text=True, encoding='utf-8', timeout=self.timeout, check=False)
            report = json.loads(result.stdout)
        except (OSError, subprocess.TimeoutExpired, ValueError) as error:
            raise BackendError('RSPDL CLI did not return a structured result') from error
        if not isinstance(report, dict):
            raise BackendError('RSPDL CLI returned an unexpected result')
        return result.returncode, report

    def schema(self):
        return {'source': self.source, 'compilation': self.compilation}

    def supported_schema(self):
        """Match this example's UI projection; leave rule evaluation to the CLI."""
        module = self.compilation.get('module') or {}
        models = module.get('models', [])
        if len(models) != 1:
            return False
        model = models[0]
        fields = model.get('fields', [])
        if len(fields) != 2 or not all(
            any(f.get('local_id') == local_id and f.get('required') is True
                and f.get('value_type', {}).get('kind') == 'date' for f in fields)
            for local_id in ('start_date', 'end_date')
        ):
            return False
        ids = {field['id'] for field in fields}
        constraints = module.get('constraints', [])
        return bool(constraints) and all(
            rule.get('model_id') == model['id']
            and rule.get('left', {}).get('kind') == 'field'
            and rule.get('right', {}).get('kind') == 'field'
            and rule['left'].get('value') in ids
            and rule['right'].get('value') in ids
            and rule['left']['value'] != rule['right']['value']
            and rule.get('operator') in {
                'less_than', 'less_than_or_equal',
                'greater_than', 'greater_than_or_equal',
            }
            for rule in constraints
        )

    def check(self, record, save=False):
        if not isinstance(record, dict) or '$id' in record:
            raise ValueError('record must be an object without a client-supplied $id')
        models = (self.compilation.get('module') or {}).get('models', [])
        # A failed compilation still gets a real CLI CheckReport, never an invented one.
        model_id = models[0]['id'] if len(models) == 1 else 'project_schedule.project'
        candidate = {'records': {model_id: [{'$id': 'project-1', **record}]}}
        with tempfile.TemporaryDirectory(prefix='rspdl-check-') as directory:
            data = Path(directory) / 'candidate.json'
            data.write_text(json.dumps(candidate, ensure_ascii=False), encoding='utf-8')
            code, report = self.invoke('check', data)
        compilation = report.get('compilation')
        accepted = (
            self.compile_code == 0 and code == 0
            and isinstance(compilation, dict) and compilation.get('module') is not None
            and isinstance(compilation.get('diagnostics'), list)
            and not any(d.get('severity') == 'error' for d in compilation['diagnostics'])
            and isinstance(report.get('runtime_diagnostics'), list)
            and not any(d.get('severity') == 'error' for d in report['runtime_diagnostics'])
            and report.get('constraint_violations') == []
            and isinstance(report.get('policy_results'), list)
            and all(p.get('status') == 'allowed' for p in report['policy_results'])
            and self.supported_schema()
        )
        if accepted and save:
            self.persist(candidate)
        return {'accepted': accepted, 'saved': accepted and save, 'report': report}

    def persist(self, candidate):
        self.data_file.parent.mkdir(parents=True, exist_ok=True)
        descriptor, name = tempfile.mkstemp(prefix='.project-', dir=self.data_file.parent)
        try:
            with os.fdopen(descriptor, 'w', encoding='utf-8') as output:
                json.dump(candidate, output, ensure_ascii=False, indent=2)
                output.write('\n')
                output.flush()
                os.fsync(output.fileno())
            os.replace(name, self.data_file)
        finally:
            if os.path.exists(name):
                os.unlink(name)


def handler_for(app):
    class Handler(BaseHTTPRequestHandler):
        def respond(self, status, payload):
            body = json.dumps(payload, ensure_ascii=False).encode('utf-8')
            self.send_response(status)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if self.path == '/api/schema':
                return self.respond(200, app.schema())
            static = {'/': ('index.html', 'text/html'), '/index.html': ('index.html', 'text/html'), '/app.mjs': ('app.mjs', 'text/javascript'), '/projection.mjs': ('projection.mjs', 'text/javascript')}
            if self.path not in static:
                return self.respond(404, {'error': 'Not found'})
            filename, mime = static[self.path]
            try:
                body = (HERE / filename).read_bytes()
            except OSError:
                return self.respond(404, {'error': 'Not found'})
            self.send_response(200)
            self.send_header('Content-Type', mime + '; charset=utf-8')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            if self.path not in ('/api/check', '/api/save'):
                return self.respond(404, {'error': 'Not found'})
            try:
                length = int(self.headers.get('Content-Length', '0'))
                if not 0 < length <= MAX_BODY or self.headers.get('Transfer-Encoding'):
                    return self.respond(413, {'error': 'Request size must be between 1 and 65536 bytes'})
                payload = json.loads(self.rfile.read(length))
                if not isinstance(payload, dict) or set(payload) != {'record'}:
                    raise ValueError('Expected {record: object}')
                result = app.check(payload['record'], save=self.path == '/api/save')
                self.respond(200, result)
            except (ValueError, UnicodeError) as error:
                self.respond(400, {'error': str(error)})
            except (BackendError, OSError):
                self.respond(503, {'error': 'RSPDL check or persistence failed; record was not accepted'})
    return Handler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, default=8765)
    parser.add_argument('--source', type=Path, default=HERE / 'project.rspdl')
    parser.add_argument('--rspdl', type=Path, default=HERE.parent.parent / 'target/debug/rspdl')
    parser.add_argument('--data-file', type=Path, default=HERE / 'saved-project.json')
    args = parser.parse_args()
    app = Application(args.source, args.rspdl, args.data_file)
    try:
        with HTTPServer(('127.0.0.1', args.port), handler_for(app)) as server:
            print(f'Project schedule demo: http://127.0.0.1:{server.server_port}', flush=True)
            server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        app.close()


if __name__ == '__main__':
    main()

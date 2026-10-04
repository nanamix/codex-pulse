#!/usr/bin/env python3
"""추론 요청 없이 App Server 초기화 및 사용 한도 조회만 검사한다. 계정 원문을 출력하지 않는다."""
import argparse, json, os, selectors, subprocess, time
parser = argparse.ArgumentParser()
parser.add_argument('--codex-path', default='/opt/homebrew/bin/codex')
parser.add_argument('--runtime-dir')
args = parser.parse_args()
command = [args.codex_path, 'app-server', '--listen', 'stdio://']
environment = os.environ.copy()
if args.runtime_dir:
    os.makedirs(args.runtime_dir, exist_ok=True)
    environment['CODEX_SQLITE_HOME'] = os.path.abspath(args.runtime_dir)
    command.extend(['-c', 'log_dir=' + json.dumps(os.path.abspath(args.runtime_dir))])
p = subprocess.Popen(command, env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
selector = selectors.DefaultSelector(); selector.register(p.stdout, selectors.EVENT_READ)
buffer = b''
def send(value):
    p.stdin.write((json.dumps(value) + '\n').encode()); p.stdin.flush()
def request(identifier, method, params):
    global buffer
    send({'id': identifier, 'method': method, 'params': params})
    expires = time.monotonic() + 20
    while time.monotonic() < expires:
        while b'\n' in buffer:
            line, buffer = buffer.split(b'\n', 1)
            v = json.loads(line)
            if v.get('id') == identifier:
                if 'error' in v:
                    raise RuntimeError('RPC error code=' + str(v['error'].get('code')))
                return v['result']
        if selector.select(max(0, expires - time.monotonic())):
            data = os.read(p.stdout.fileno(), 65536)
            if not data: raise RuntimeError('App Server disconnected')
            buffer += data
    raise RuntimeError('Request timed out')
code = 0
try:
    request(1, 'initialize', {'clientInfo': {'name': 'codex_usage_probe', 'version': '0.1.0'}})
    print('initialize: OK')
    send({'method': 'initialized'})
    account = request(2, 'account/read', {'refreshToken': False})
    print('account/read: OK; authType=' + str((account.get('account') or {}).get('type')))
    limits = request(3, 'account/rateLimits/read', {'excludeResetCreditDetails': True})
    print('account/rateLimits/read: OK')
    buckets = limits.get('rateLimitsByLimitId') or {'default': limits.get('rateLimits', {})}
    print('bucketCount=' + str(len(buckets)))
    for identifier, b in buckets.items():
        print(json.dumps({'limitId': identifier, 'primary': b.get('primary'), 'secondary': b.get('secondary')}, ensure_ascii=False))
except Exception as error:
    print('probe: FAILED; ' + str(error)); code = 1
finally:
    p.terminate()
    try: p.wait(timeout=2)
    except subprocess.TimeoutExpired: p.kill(); p.wait()
    selector.close()
    print('childCleanup: OK')
raise SystemExit(code)

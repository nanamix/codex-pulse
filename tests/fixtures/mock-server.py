#!/usr/bin/env python3
import json, os, sys, time
mode = os.environ.get('MOCK_MODE', 'normal')
initialized = False
count = 0
rate_calls = 0
for line in sys.stdin:
    req = json.loads(line)
    method = req.get('method')
    if method == 'initialized':
        initialized = True
        continue
    if 'id' not in req:
        continue
    result = {}
    if method == 'initialize':
        result = {'userAgent': 'mock'}
    elif not initialized:
        print(json.dumps({'id': req['id'], 'error': {'code': -32000, 'message': 'not initialized'}}), flush=True)
        continue
    elif method == 'account/read':
        count += 1
        if mode == 'startup-account-notify':
            print(json.dumps({'method': 'account/updated', 'params': {'authMode': 'chatgpt'}}), flush=True)
        result = {'account': None if mode == 'login' else {'type': 'apiKey' if mode == 'apikey' else 'chatgpt', 'email': 'second@example.invalid' if mode == 'account-change' and count > 1 else 'first@example.invalid'}, 'requiresOpenaiAuth': True}
    elif method == 'account/rateLimits/read':
        rate_calls += 1
        if mode == 'mid-read-account-notify':
            print(json.dumps({'method': 'account/updated', 'params': {'authMode': 'chatgpt'}}), flush=True)
        if mode == 'slow-read':
            time.sleep(.25)
        if mode == 'timeout' and count > 1:
            time.sleep(30)
        if mode == 'account-change' and count > 1:
            print(json.dumps({'id': req['id'], 'error': {'code': -32001, 'message': 'secret-token-never-log'}}), flush=True)
            continue
        bucket = {'limitId': 'codex', 'primary': {'usedPercent': 25, 'windowDurationMins': 15, 'resetsAt': 1730947200}, 'secondary': None}
        # A notification and a stray response must never satisfy this request.
        print(json.dumps({'method': 'account/rateLimits/updated', 'params': {'rateLimits': {**bucket, 'primary': {**bucket['primary'], 'usedPercent': 31}}}}), flush=True)
        print(json.dumps({'id': 99999, 'result': {'wrong': True}}), flush=True)
        if mode == 'slow-read':
            bucket['credits'] = {'balance': str(rate_calls)}
        result = {'rateLimits': bucket, 'rateLimitResetCredits': {'availableCount': 7, 'credits': [{'status': 'available', 'expiresAt': 1730947200}] if req.get('params', {}).get('excludeResetCreditDetails') is False else None}}
        if mode == 'workspace-change':
            result['accountId'] = 'workspace-a' if count == 1 else 'workspace-b'
    elif method == 'account/usage/read':
        if mode == 'post-read-notify':
            print(json.dumps({'method': 'account/rateLimits/updated', 'params': {'rateLimits': {'limitId': 'codex', 'primary': {'usedPercent': 31, 'windowDurationMins': 15}}}}), flush=True)
        if mode == 'workspace-change':
            result = {'summary': {'lifetimeTokens': count * 100}}
        else:
            print(json.dumps({'id': req['id'], 'error': {'code': -32601, 'message': 'unsupported'}}), flush=True)
            continue
    elif method == 'mock/error':
        print(json.dumps({'id': req['id'], 'error': {'code': -32000, 'message': 'secret-token-never-log'}}), flush=True)
        continue
    print(json.dumps({'id': req['id'], 'result': result}), flush=True)

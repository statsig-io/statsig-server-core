from statsig_python_core import Statsig, StatsigUser, StatsigOptions, ObservabilityClient
import os
import platform
import sys
import json

async def main():
    sdk_key = os.environ.get('STATSIG_SERVER_SDK_KEY')
    if not sdk_key:
        raise Exception('STATSIG_SERVER_SDK_KEY is not set')

    statsig = Statsig(sdk_key)
    statsig.initialize().wait()

    user = StatsigUser(user_id='a_user')
    user.custom = {
        'os': platform.system().lower(),
        'arch': platform.machine(),
        'pythonVersion': platform.python_version(),
    }

    gate = statsig.check_gate(user, 'test_public')
    gcir = statsig.get_client_initialize_response(user)

    print('-------------------------------- Get Client Initialize Response --------------------------------')
    print(json.dumps(json.loads(gcir), indent=2))
    print('-------------------------------------------------------------------------------------------------')

    print('Gate test_public: ', gate)

    if not gate:
        raise Exception('"test_public" gate is false but should be true')

    gcir_json = json.loads(gcir)
    if len(gcir_json.keys()) < 1:
        raise Exception('GCIR is missing required fields')

    print('All checks passed, shutting down...')
    statsig.shutdown().wait()
    print('Shutdown complete')

if __name__ == '__main__':
    import asyncio
    try:
        asyncio.run(main())
    except Exception as e:
        print('Error:', str(e), file=sys.stderr)
        sys.exit(1)
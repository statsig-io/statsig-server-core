<?php

require_once "../vendor/autoload.php";

use Statsig\Statsig;
use Statsig\StatsigUser;
use Statsig\StatsigOptions;

function main()
{
    $options = new StatsigOptions(
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        "debug"
    );

    $secret_key = getenv('test_api_key');
    $statsig = new Statsig($secret_key, $options);
    $statsig->initialize();

    $user = new StatsigUser("a-user", [], "daniel@statsig.com");
    $gcir = $statsig->getClientInitializeResponse($user);

    echo time() . "\n";
    echo strlen($gcir) . "\n";

    $statsig->shutdown();
}

//phpinfo();
for ($i = 0; $i < 10; ++$i) {
    main();
}

sleep(1);

echo "done";
// $statsig->shutdown();

// while (true) {
//     sleep(10);
// }

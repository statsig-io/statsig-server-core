require 'statsig'
require 'benchmark'

Statsig.initialize('secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW')

user = StatsigUser.new({'userID' => 'Dan'})

time = Benchmark.measure {
    init_res = {}
    1000.times do
        init_res = Statsig.get_client_initialize_response(user)
    end 
    puts "Client init res: #{init_res}"
}

puts (time.real * 1000) 
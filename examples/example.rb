require_relative '../build/ruby/statsig'
require 'benchmark'

name = "Dan Smith"
email = "daniel@statsig.com"

user = User.create(name, email)
statsig = Statsig.for_user(user, "secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW")

# gate_name = "example_gate"
# result = statsig.check_gate(gate_name)
# puts "Gate check #{result ? 'passed' : 'failed'}."

time = Benchmark.measure {
    init_res = {}
    1000.times do
        init_res = statsig.get_client_init_res
    end 
    puts "Client init res: #{init_res}"
}
puts (time.real * 1000) 

Statsig.destroy(statsig)
User.destroy(user)

require 'bundler/setup'
require 'statsig'
require 'benchmark'

Statsig.initialize( ENV['PERF_SDK_KEY'])

SDK_TYPE = 'ruby-server'
SDK_VERSION = Gem.loaded_specs['statsig'].version.to_s

METADATA_FILE = ENV['BENCH_METADATA_FILE']
File.write(METADATA_FILE, {
  sdk_type: SDK_TYPE,
  sdk_version: SDK_VERSION
}.to_json)

CORE_ITER = 100_000
GCIR_ITER = 1000

GLOBAL_USER = StatsigUser.new(user_id: "global_user")

RESULTS = {}

def log_benchmark(name, p99)
  puts "#{name.ljust(50)} #{p99.round(4)}ms"

  if ENV['CI'] != '1' && ENV['CI'] != 'true'
    return
  end

  Statsig.log_event(
    GLOBAL_USER,
    'sdk_benchmark',
    p99,
    {
      "benchmarkName" => name,
      "sdkType" => SDK_TYPE,
      "sdkVersion" => SDK_VERSION
    }
  )
end

def make_random_user
  StatsigUser.new(user_id: "user_#{rand(1_000_000)}")
end

def benchmark(iterations, &block)
  durations = []
  
  iterations.times do
    start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    block.call
    end_time = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    durations << (end_time - start) * 1000 # Convert to ms
  end

  # Calculate p99
  sorted_durations = durations.sort
  index = (0.99 * sorted_durations.length).ceil - 1
  sorted_durations[index]
end

def run_check_gate
  p99 = benchmark(CORE_ITER) do
    user = make_random_user
    Statsig.check_gate(user, "test_advanced")
  end
  RESULTS["check_gate"] = p99
end

def run_check_gate_global_user
  p99 = benchmark(CORE_ITER) do
    Statsig.check_gate(GLOBAL_USER, "test_advanced")
  end
  RESULTS["check_gate_global_user"] = p99
end

# Unsupported: get_feature_gate is not implemented in ruby
# def run_get_feature_gate
#   p99 = benchmark do
#     user = make_random_user
#     Statsig.get_feature_gate(user, "test_advanced")
#   end
#   RESULTS["get_feature_gate"] = p99
# end
#
# def run_get_feature_gate_global_user
#   p99 = benchmark do
#     Statsig.get_feature_gate(GLOBAL_USER, "test_advanced")
#   end
#   RESULTS["get_feature_gate_global_user"] = p99
# end

def run_get_experiment
  p99 = benchmark(CORE_ITER) do
    user = make_random_user
    Statsig.get_experiment(user, "an_experiment")
  end
  RESULTS["get_experiment"] = p99
end

def run_get_experiment_global_user
  p99 = benchmark(CORE_ITER) do
    Statsig.get_experiment(GLOBAL_USER, "an_experiment")
  end
  RESULTS["get_experiment_global_user"] = p99
end

def run_get_client_initialize_response
  p99 = benchmark(GCIR_ITER) do
    user = make_random_user
    Statsig.get_client_initialize_response(user)
  end
  RESULTS["get_client_initialize_response"] = p99
end

def run_get_client_initialize_response_global_user
  p99 = benchmark(GCIR_ITER) do
    Statsig.get_client_initialize_response(GLOBAL_USER)
  end
  RESULTS["get_client_initialize_response_global_user"] = p99
end

puts "Statsig Ruby Legacy (v#{SDK_VERSION})"
puts "--------------------------------"

run_check_gate
run_check_gate_global_user
# run_get_feature_gate
# run_get_feature_gate_global_user
run_get_experiment
run_get_experiment_global_user
run_get_client_initialize_response
run_get_client_initialize_response_global_user

RESULTS.each do |name, p99|
  log_benchmark(name, p99)
end

Statsig.shutdown
puts "\n\n"

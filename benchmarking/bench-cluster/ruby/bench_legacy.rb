require 'statsig'
require 'json'

SDK_TYPE = "ruby-server"
SDK_VERSION = Gem.loaded_specs['statsig'].version.to_s

ITER_LITE = 1000
ITER_HEAVY = 10_000

class BenchLegacy
    def self.run
        puts "Statsig Ruby Legacy (v#{SDK_VERSION})"
        puts "--------------------------------"

        spec_names = load_spec_names

        options = StatsigOptions.new(
            download_config_specs_url: "http://scrapi:8000/v2/download_config_specs",
            log_event_url: "http://scrapi:8000/v1/log_event",
        )

        statsig = Statsig.initialize("secret-RUBY_LEGACY", options)
        global_user = StatsigUser.new({'userID' => "global_user"})

        results = []

        spec_names["feature_gates"].each do |gate|
            results << benchmark("check_gate", gate, ITER_HEAVY) do
                user = create_user
                Statsig.check_gate(user, gate)
            end

            results << benchmark("check_gate_global_user", gate, ITER_HEAVY) do
                Statsig.check_gate(global_user, gate)
            end
        end

        spec_names["dynamic_configs"].each do |config|
            results << benchmark("get_dynamic_config", config, ITER_HEAVY) do
                user = create_user
                Statsig.get_config(user, config)
            end

            results << benchmark("get_dynamic_config_global_user", config, ITER_HEAVY) do
                Statsig.get_config(global_user, config)
            end
        end

        spec_names["experiments"].each do |experiment|
            results << benchmark("get_experiment", experiment, ITER_HEAVY) do
                user = create_user
                Statsig.get_experiment(user, experiment)
            end

            results << benchmark("get_experiment_global_user", experiment, ITER_HEAVY) do
                Statsig.get_experiment(global_user, experiment)
            end
        end

        spec_names["layers"].each do |layer|
            results << benchmark("get_layer", layer, ITER_HEAVY) do
                user = create_user
                Statsig.get_layer(user, layer)
            end

            results << benchmark("get_layer_global_user", layer, ITER_HEAVY) do
                Statsig.get_layer(global_user, layer)
            end
        end

        results << benchmark("get_client_initialize_response", "n/a", ITER_LITE) do
            user = create_user
            Statsig.get_client_initialize_response(user)
        end
        
        results << benchmark("get_client_initialize_response_global_user", "n/a", ITER_LITE) do
            Statsig.get_client_initialize_response(global_user)
        end
        

        Statsig.shutdown

        write_results(results)
    end

    def self.load_spec_names
        for i in 0..10
            if File.exist?("/shared-volume/spec_names.json")
                break
            end
            sleep(1)
        end

        json = File.read("/shared-volume/spec_names.json")
        JSON.parse(json)
    end

    def self.create_user
        StatsigUser.new({
            'userID' => "user_#{rand(1_000_000)}", 
            'email' => "user@example.com", 
            'ipAddress' => "127.0.0.1", 
            'locale' => "en-US", 
            'country' => "US", 
            'appVersion' => "1.0.0", 
            'userAgent' => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            'custom' => {
                'isAdmin' => false,
            },
            'privateAttributes' => {
                'isPaid' => "nah",
            },
        })
    end

    def self.benchmark(bench_name, spec_name, iterations, &block)
        durations = []
        
        iterations.times do
          start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
          block.call
          end_time = Process.clock_gettime(Process::CLOCK_MONOTONIC)
          durations << (end_time - start) * 1000 # Convert to ms
        end
      
        sorted_durations = durations.sort
        result = {
            benchmarkName: bench_name,
            specName: spec_name,
            p99: sorted_durations[0.99 * sorted_durations.length],
            max: sorted_durations[sorted_durations.length - 1],
            min: sorted_durations[0],
            median: sorted_durations[sorted_durations.length / 2],
            avg: sorted_durations.sum / sorted_durations.length,
            sdkType: SDK_TYPE,
            sdkVersion: SDK_VERSION,
        }

        puts "#{result[:benchmarkName].ljust(30)} p99(#{result[:p99].round(4)}ms) max(#{result[:max].round(4)}ms) #{result[:specName]}"

        result
    end

    def self.write_results(results)
        pretty_results = JSON.pretty_generate({
            sdkType: SDK_TYPE,
            sdkVersion: SDK_VERSION,
            results: results,
        })
        File.write("/shared-volume/#{SDK_TYPE}-#{SDK_VERSION}-results.json", pretty_results)
    end
end
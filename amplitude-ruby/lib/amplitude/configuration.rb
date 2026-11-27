# frozen_string_literal: true

require 'logger'
require 'uri'

class Amplitude
  class Configuration
    LOG_LEVELS = [Logger::UNKNOWN, Logger::ERROR, Logger::WARN, Logger::INFO, Logger::DEBUG].freeze
    private_constant :LOG_LEVELS

    attr_reader :api_key, :opt_out, :server_url, :server_zone, :use_batch, :timeout_ms,
                :flush_interval_millis, :flush_queue_size, :flush_max_retries, :plan,
                :ingestion_metadata, :log_level, :instance_name, :logger, :upload_uri

    def initialize(api_key, config)
      unless api_key.is_a?(String) && !api_key.strip.empty?
        raise Error, 'api_key must be a nonblank string'
      end

      @api_key = api_key.dup
      config = Payload.snapshot(config).compact
      @opt_out = boolean(config.fetch('opt_out', false), 'opt_out')
      @use_batch = boolean(config.fetch('use_batch', false), 'use_batch')
      @server_zone = config.fetch('server_zone', 'us')
      raise Error, 'server_zone must be us or eu' unless %w[us eu].include?(@server_zone)

      default_host = @server_zone == 'eu' ? 'api.eu.amplitude.com' : 'api2.amplitude.com'
      default_path = @use_batch ? '/batch' : '/2/httpapi'
      @server_url = config.fetch('server_url', "https://#{default_host}#{default_path}")
      @upload_uri = endpoint(@server_url)
      @timeout_ms = config.fetch('timeout_ms', 10_000)
      unless @timeout_ms.is_a?(Integer) && (1...2**64).cover?(@timeout_ms)
        raise Error, 'timeout_ms must be a positive integer'
      end

      interval = config.fetch('flush_interval_millis', 0)
      @flush_interval_millis = interval.to_f if interval.is_a?(Numeric)
      unless @flush_interval_millis&.finite? && @flush_interval_millis >= 0
        raise Error, 'flush_interval_millis must be a finite nonnegative duration'
      end

      @flush_queue_size = count(config.fetch('flush_queue_size', 200), 1, 'flush_queue_size')
      @flush_max_retries = count(config.fetch('flush_max_retries', 0), 0, 'flush_max_retries')
      @log_level = config.fetch('log_level', 0)
      unless @log_level.is_a?(Integer) && (0...LOG_LEVELS.length).cover?(@log_level)
        raise Error, 'log_level must be an integer between 0 and 4'
      end

      @instance_name = config['instance_name']
      if !@instance_name.nil? && !@instance_name.is_a?(String)
        raise Error, 'instance_name must be a string'
      end

      @plan = Payload.metadata(config['plan'], 'plan')
      @ingestion_metadata = Payload.metadata(config['ingestion_metadata'], 'ingestion_metadata')
      @logger = Logger.new($stderr, level: LOG_LEVELS.fetch(@log_level), progname: @instance_name || 'Amplitude')
    end

    private

    def endpoint(url)
      uri = URI.parse(url) if url.is_a?(String)
      return uri if uri.is_a?(URI::HTTP) && uri.host && !uri.host.empty? && (1..65_535).cover?(uri.port)

      raise Error, 'server_url must be a valid HTTP(S) URL'
    rescue URI::Error
      raise Error, 'server_url must be a valid HTTP(S) URL'
    end

    def boolean(value, name)
      return value if value == true || value == false

      raise Error, "#{name} must be a boolean"
    end

    def count(value, minimum, name)
      if value.is_a?(Numeric) && (minimum...2**32).cover?(value) && value == value.to_i
        return value.to_i
      end

      raise Error, "#{name} must be a #{minimum.zero? ? 'nonnegative' : 'positive'} whole number"
    end
  end
end

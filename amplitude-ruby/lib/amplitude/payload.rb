# frozen_string_literal: true

require 'json'

class Amplitude
  module Payload
    EVENT_FIELDS = %w[event_type event_properties user_properties groups group_properties library].freeze
    OBJECT_FIELDS = %w[event_properties user_properties group_properties groups].freeze
    STRING_OPTIONS = %w[
      user_id device_id app_version platform os_name os_version device_brand device_manufacturer
      device_model carrier country region city dma language product_id revenue_type currency ip
      idfa idfv adid android_id insert_id partner_id version_name user_agent
    ].freeze
    INTEGER_OPTIONS = %w[time quantity event_id session_id].freeze
    NUMBER_OPTIONS = %w[price revenue location_lat location_lng].freeze
    METADATA_FIELDS = {
      'plan' => %w[branch source version version_id].freeze,
      'ingestion_metadata' => %w[source_name source_version].freeze
    }.freeze
    WIRE_NAMES = { 'product_id' => 'productId', 'revenue_type' => 'revenueType' }.freeze

    def self.snapshot(value)
      JSON.parse(JSON.generate(value))
    rescue JSON::GeneratorError, JSON::ParserError => error
      raise Error, "Invalid JSON data: #{error.message}"
    end

    def self.object(value, name)
      raise Error, "#{name} must be an object" unless value.is_a?(Hash)

      value
    end

    def self.metadata(data, name)
      return if data.nil?

      fields = object(data, name).slice(*METADATA_FIELDS.fetch(name)).compact
      fields.each { |field, value| string(value, "#{name}.#{field}") }
      fields
    end

    def self.event(data)
      data = snapshot(data)
      fields = data.slice(*EVENT_FIELDS).compact
      raise Error, 'event_type is required' unless fields.key?('event_type')

      string(fields['event_type'], 'event_type')
      string(fields['library'], 'library') if fields.key?('library')
      OBJECT_FIELDS.each { |key| object(fields[key], key) if fields.key?(key) }
      (fields['groups'] || {}).each_value do |names|
        next if names.is_a?(String) || (names.is_a?(Array) && names.all?(String))

        raise Error, 'Group names must be strings or arrays of strings'
      end
      fields['options'] = event_options(data.fetch('options'))
      fields
    end

    def self.validate_event(event)
      raise Error, 'event_type is required' if event.fetch('event_type').strip.empty?

      ids = event.fetch('options').values_at('user_id', 'device_id')
      raise Error, 'user_id or device_id is required' unless ids.any? { |id| id && !id.strip.empty? }
    end

    def self.event_options(data)
      data = object(data, 'options').dup
      WIRE_NAMES.each do |name, wire|
        next unless data.key?(wire)

        raise Error, "Duplicate event option #{name}" if data.key?(name)

        data[name] = data.delete(wire)
      end
      fields = data.slice(*STRING_OPTIONS, *INTEGER_OPTIONS, *NUMBER_OPTIONS, *METADATA_FIELDS.keys).compact
      STRING_OPTIONS.each { |key| string(fields[key], key) if fields.key?(key) }
      INTEGER_OPTIONS.each do |key|
        next unless fields.key?(key)

        value = fields[key]
        unless value.is_a?(Integer) && (-(2**63)...(2**63)).cover?(value)
          raise Error, "#{key} must be a signed 64-bit integer"
        end
      end
      NUMBER_OPTIONS.each do |key|
        next unless fields.key?(key)

        value = fields[key]
        raise Error, "#{key} must be a finite number" unless value.is_a?(Numeric) && value.finite?
      end
      METADATA_FIELDS.each_key { |key| fields[key] = metadata(fields[key], key) if fields.key?(key) }
      fields
    end

    def self.string(value, name)
      raise Error, "#{name} must be a string" unless value.is_a?(String)
    end

    private_class_method :event_options, :string
  end

  private_constant :Payload
end

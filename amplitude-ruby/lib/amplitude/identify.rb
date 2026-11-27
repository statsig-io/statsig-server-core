# frozen_string_literal: true

class Amplitude
  class Identify
    OPERATORS = {
      'set' => '$set', 'set_once' => '$setOnce', 'append' => '$append',
      'prepend' => '$prepend', 'pre_insert' => '$preInsert', 'post_insert' => '$postInsert',
      'remove' => '$remove', 'add' => '$add', 'unset' => '$unset'
    }.freeze
    private_constant :OPERATORS

    def initialize
      @operations = []
    end

    OPERATORS.each_key do |operation|
      next if operation == 'unset'

      define_method(operation) do |key, value|
        @operations << { operation: operation.to_sym, key: key, value: value }
        self
      end
    end

    def unset(key)
      @operations << { operation: :unset, key: key }
      self
    end

    def clear_all
      @operations << { operation: :clear_all }
      self
    end

    def to_a
      @operations.dup
    end

    def self.user_properties(operations)
      unless operations.respond_to?(:to_a)
        raise Error, 'Identify operations must be a list'
      end

      operations = Payload.snapshot(operations.to_a)
      raise Error, 'At least one identify operation is required' if operations.empty?

      operations.each { |operation| Payload.object(operation, 'Identify operation') }
      if operations.any? { |operation| operation['operation'] == 'clear_all' }
        raise Error, 'Clear-all must be the only identify operation' unless operations.length == 1

        return { '$clearAll' => '-' }
      end

      keys = {}
      properties = {}
      operations.each do |operation|
        operator = OPERATORS[operation['operation']]
        raise Error, 'Unknown identify operation' if operator.nil?

        key = operation['key']
        raise Error, 'Identify property names must not be empty' unless key.is_a?(String) && !key.strip.empty?
        raise Error, 'Supply one identify operation per property' if keys.key?(key)

        keys[key] = true
        value = operation_value(operation)
        (properties[operator] ||= {})[key] = value
      end
      properties
    end

    def self.operation_value(operation)
      return '-' if operation['operation'] == 'unset'

      raise Error, 'Identify operation requires a value' unless operation.key?('value')

      value = operation['value']
      if operation['operation'] == 'add' && !(value.is_a?(Numeric) && value.finite?)
        raise Error, 'The identify add value must be finite'
      end

      value
    end

    private_class_method :operation_value
  end
end

# frozen_string_literal: true

require 'json'
require_relative 'auth/version'
require_relative 'auth/auth'

module Auth
end

module SmbCloud
  module Auth
    class Error < StandardError
      attr_reader :error_code, :payload

      def initialize(message = nil, error_code: nil, payload: nil)
        super(message)
        @error_code = error_code
        @payload = payload
      end
    end

    module Environment
      DEV = 'dev'
      PRODUCTION = 'production'
    end

    class << self
      def signup_with_client(environment:, app_id:, app_secret:, email:, password:)
        parse_json(__signup_with_client(environment, app_id, app_secret, email, password))
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def login_with_client(environment:, app_id:, app_secret:, email:, password:)
        parse_json(__login_with_client(environment, app_id, app_secret, email, password))
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def me_with_client(environment:, app_id:, app_secret:, access_token:)
        parse_json(__me_with_client(environment, app_id, app_secret, access_token))
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def logout_with_client(environment:, app_id:, app_secret:, access_token:)
        __logout_with_client(environment, app_id, app_secret, access_token)
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def remove_with_client(environment:, app_id:, app_secret:, access_token:)
        __remove_with_client(environment, app_id, app_secret, access_token)
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def reset_password(environment:, app_id:, token:, password:)
        parse_json(__reset_password(environment, app_id, token, password))
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def resend_email_verification(environment:, app_id:, email:)
        parse_json(__resend_email_verification(environment, app_id, email))
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      def resend_reset_password_instruction(environment:, app_id:, email:)
        parse_json(__resend_reset_password_instruction(environment, app_id, email))
      rescue RuntimeError => e
        raise normalize_error(e)
      end

      private

      def parse_json(payload)
        return payload unless payload.is_a?(String)

        JSON.parse(payload, symbolize_names: true)
      end

      def normalize_error(error)
        payload = parse_json(error.message)
        error_hash = payload[:error] || payload
        message = error_hash[:message] || error.message
        error_code = error_hash[:error_code]
        Error.new(message, error_code: error_code, payload: payload)
      rescue JSON::ParserError, NoMethodError, TypeError
        Error.new(error.message)
      end
    end
  end
end

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

    class Client
      attr_reader :environment, :app_id, :app_secret

      def initialize(environment:, app_id:, app_secret:)
        @environment = environment
        @app_id = app_id
        @app_secret = app_secret
      end

      def signup(email:, password:)
        Auth.send(:parse_json, Auth.__signup_with_client(environment, app_id, app_secret, email, password))
      rescue RuntimeError => e
        raise Auth.send(:normalize_error, e)
      end

      def login(email:, password:)
        Auth.send(:parse_json, Auth.__login_with_client(environment, app_id, app_secret, email, password))
      rescue RuntimeError => e
        raise Auth.send(:normalize_error, e)
      end

      def me(access_token:)
        Auth.send(:parse_json, Auth.__me_with_client(environment, app_id, app_secret, access_token))
      rescue RuntimeError => e
        raise Auth.send(:normalize_error, e)
      end

      def logout(access_token:)
        Auth.__logout_with_client(environment, app_id, app_secret, access_token)
      rescue RuntimeError => e
        raise Auth.send(:normalize_error, e)
      end

      def remove(access_token:)
        Auth.__remove_with_client(environment, app_id, app_secret, access_token)
      rescue RuntimeError => e
        raise Auth.send(:normalize_error, e)
      end
    end

    class << self
      def client(environment:, app_id:, app_secret:)
        Client.new(environment:, app_id:, app_secret:)
      end

      def signup_with_client(environment:, app_id:, app_secret:, email:, password:)
        client(environment:, app_id:, app_secret:).signup(email:, password:)
      end

      def login_with_client(environment:, app_id:, app_secret:, email:, password:)
        client(environment:, app_id:, app_secret:).login(email:, password:)
      end

      def me_with_client(environment:, app_id:, app_secret:, access_token:)
        client(environment:, app_id:, app_secret:).me(access_token:)
      end

      def logout_with_client(environment:, app_id:, app_secret:, access_token:)
        client(environment:, app_id:, app_secret:).logout(access_token:)
      end

      def remove_with_client(environment:, app_id:, app_secret:, access_token:)
        client(environment:, app_id:, app_secret:).remove(access_token:)
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

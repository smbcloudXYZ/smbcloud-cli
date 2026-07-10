# frozen_string_literal: true

require 'json'
require_relative 'email/version'
require_relative 'email/email'

module Email
end

module SmbCloud
  # Ruby bindings for the smbCloud transactional email API, powered by the shared
  # Rust SDK (smbcloud-email-sdk) and a native Magnus extension.
  #
  # @example
  #   client = SmbCloud::Email.client(environment: "production", api_key: "smb_mail_…")
  #   sent = client.send(
  #     from: "billing@example.com",
  #     to: ["customer@acme.com"],
  #     subject: "Your receipt",
  #     html: "<h1>Thanks!</h1>",
  #     idempotency_key: "receipt-2026-0001",
  #   )
  #   sent[:id] # => "eml_…"
  module Email
    class Error < StandardError
      attr_reader :payload

      def initialize(message = nil, payload: nil)
        super(message)
        @payload = payload
      end
    end

    module Environment
      DEV = 'dev'
      PRODUCTION = 'production'
    end

    class Client
      attr_reader :environment, :api_key

      def initialize(environment:, api_key:)
        @environment = environment
        @api_key = api_key
      end

      # Send a transactional email. `from` must be on the API key's verified
      # domain; provide at least one of `html`/`text`. `attachments` is an array
      # of { filename:, content_base64: }. Returns the created message hash.
      def send(from:, to:, subject: nil, html: nil, text: nil, cc: nil, bcc: nil,
               reply_to: nil, attachments: nil, headers: nil, tags: nil,
               idempotency_key: nil)
        message = {
          from: from,
          to: Array(to),
          cc: Array(cc),
          bcc: Array(bcc),
          reply_to: Array(reply_to),
          subject: subject,
          html: html,
          text: text,
          attachments: Array(attachments),
          headers: headers || {},
          tags: tags || {},
          idempotency_key: idempotency_key
        }.compact

        Email.send(:parse_json, Email.__send(environment, api_key, JSON.generate(message)))
      rescue RuntimeError => e
        raise Email.send(:normalize_error, e)
      end

      # Fetch one message by id, including its delivery-event timeline. Requires
      # a read-scope key.
      def get_message(id)
        Email.send(:parse_json, Email.__get_message(environment, api_key, id))
      rescue RuntimeError => e
        raise Email.send(:normalize_error, e)
      end

      # List recent messages. `status` filters by delivery status name; `limit`
      # is clamped server-side to 1..=100. Requires a read-scope key.
      def list_messages(status: nil, limit: nil)
        Email.send(:parse_json,
                   Email.__list_messages(environment, api_key, status.to_s, limit.to_i))
      rescue RuntimeError => e
        raise Email.send(:normalize_error, e)
      end
    end

    class << self
      def client(environment:, api_key:)
        Client.new(environment:, api_key:)
      end

      def send_with_client(environment:, api_key:, **kwargs)
        client(environment:, api_key:).send(**kwargs)
      end

      def get_message_with_client(environment:, api_key:, id:)
        client(environment:, api_key:).get_message(id)
      end

      def list_messages_with_client(environment:, api_key:, status: nil, limit: nil)
        client(environment:, api_key:).list_messages(status:, limit:)
      end

      private

      def parse_json(payload)
        return payload unless payload.is_a?(String)

        JSON.parse(payload, symbolize_names: true)
      end

      def normalize_error(error)
        payload = parse_json(error.message)
        message = payload.is_a?(Hash) ? (payload[:message] || error.message) : error.message
        Error.new(message, payload: payload)
      rescue JSON::ParserError, NoMethodError, TypeError
        Error.new(error.message)
      end
    end
  end
end

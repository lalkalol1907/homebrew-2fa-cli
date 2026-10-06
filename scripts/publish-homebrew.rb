#!/usr/bin/env ruby
require "base64"
require "json"
require "open3"
require "rubygems"

module HomebrewDelivery
  class ApiError < StandardError
    attr_reader :status

    def initialize(message, status)
      super(message)
      @status = status
    end
  end

  def self.api(method, endpoint, payload = nil)
    args = ["gh", "api", "--method", method, endpoint]
    args += ["--input", "-"] if payload
    out, err, result = Open3.capture3(*args, stdin_data: payload ? JSON.generate(payload) : "")
    raise ApiError.new(err, err[/HTTP (\d+)/, 1].to_i) unless result.success?

    JSON.parse(out)
  end

  def self.publish(formula, repository, api: method(:api))
    version = formula[/^  version "([^"]+)"$/, 1]
    raise "Generated binary formula has no version" unless version

    endpoint = "repos/#{repository}/contents/Formula/twofa.rb"
    3.times do |attempt|
      file = api.call("GET", "#{endpoint}?ref=master")
      current = Base64.decode64(file.fetch("content"))
      current_version = current[/^  version "([^"]+)"$/, 1] || current[/tag: "v([^"]+)"/, 1]
      raise "Cannot determine the current formula version" unless current_version

      return :already_delivered if current == formula
      return :newer_release if Gem::Version.new(current_version) > Gem::Version.new(version)

      begin
        api.call("PUT", endpoint, {
          "message" => "brew: deliver twofa v#{version}",
          "branch" => "master",
          "sha" => file.fetch("sha"),
          "content" => Base64.strict_encode64(formula)
        })
        return :delivered
      rescue ApiError => error
        # Re-read master if another commit changed the formula concurrently.
        raise unless error.status == 409 && attempt < 2
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  abort "Usage: ruby scripts/publish-homebrew.rb FORMULA" unless ARGV.length == 1
  repository = ENV.fetch("GITHUB_REPOSITORY")
  puts HomebrewDelivery.publish(File.read(ARGV.fetch(0)), repository)
end

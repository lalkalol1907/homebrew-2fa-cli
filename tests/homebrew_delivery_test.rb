require "minitest/autorun"
require_relative "../scripts/publish-homebrew"

class HomebrewDeliveryTest < Minitest::Test
  FORMULA = "class Twofa < Formula\n  version \"0.1.1\"\nend\n".freeze

  def file(content, sha = "current-sha")
    { "content" => Base64.strict_encode64(content), "sha" => sha }
  end

  def test_updates_only_formula_on_master_from_the_latest_file_sha
    calls = []
    api = lambda do |method, endpoint, payload = nil|
      calls << [method, endpoint, payload]
      method == "GET" ? file('url "repo.git", tag: "v0.1.0"') : {}
    end
    assert_equal :delivered, HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
    assert_equal "repos/owner/repo/contents/Formula/twofa.rb?ref=master", calls.first[1]
    payload = calls.last[2]
    assert_equal "master", payload["branch"]
    assert_equal "current-sha", payload["sha"]
    assert_equal FORMULA, Base64.decode64(payload["content"])
  end

  def test_repeated_delivery_does_not_create_a_commit
    api = lambda do |method, *_|
      assert_equal "GET", method
      file(FORMULA)
    end
    assert_equal :already_delivered, HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
  end

  def test_old_tag_cannot_downgrade_a_newer_release
    api = lambda do |method, *_|
      assert_equal "GET", method
      file("  version \"0.2.0\"\n")
    end
    assert_equal :newer_release, HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
  end

  def test_conflicting_update_reloads_the_formula_instead_of_overwriting_it
    reads = 0
    api = lambda do |method, _, payload = nil|
      if method == "GET"
        reads += 1
        file('tag: "v0.1.0"', "sha-#{reads}")
      elsif reads == 1
        raise HomebrewDelivery::ApiError.new("conflict", 409)
      else
        assert_equal "sha-2", payload["sha"]
        {}
      end
    end
    assert_equal :delivered, HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
    assert_equal 2, reads
  end

  def test_protected_branch_failure_is_reported
    api = lambda do |method, *_|
      if method == "GET"
        file('tag: "v0.1.0"')
      else
        raise HomebrewDelivery::ApiError.new("protected branch", 403)
      end
    end
    assert_raises(HomebrewDelivery::ApiError) do
      HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
    end
  end

  def test_newer_release_winning_a_conflict_prevents_downgrade
    reads = 0
    api = lambda do |method, *_|
      if method == "GET"
        reads += 1
        file(reads == 1 ? 'tag: "v0.1.0"' : "  version \"0.2.0\"\n")
      else
        raise HomebrewDelivery::ApiError.new("conflict", 409)
      end
    end
    assert_equal :newer_release, HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
    assert_equal 2, reads
  end

  def test_repeated_conflicts_fail_after_three_attempts
    writes = 0
    api = lambda do |method, *_|
      if method == "GET"
        file('tag: "v0.1.0"')
      else
        writes += 1
        raise HomebrewDelivery::ApiError.new("conflict", 409)
      end
    end
    assert_raises(HomebrewDelivery::ApiError) do
      HomebrewDelivery.publish(FORMULA, "owner/repo", api: api)
    end
    assert_equal 3, writes
  end
end

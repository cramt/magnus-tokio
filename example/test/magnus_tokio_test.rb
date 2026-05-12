# frozen_string_literal: true

require_relative "helper"

class MagnusTokioTest < Minitest::Test
  def test_sleep_returns_value
    result = Sync { MyModule.sleep(50).wait }
    assert_instance_of MyModule::ExampleStruct, result
    assert_equal 50, result.sleep_time
  end

  def test_fail_after_raises_configured_error
    err = assert_raises(MyModule::Error) do
      Sync { MyModule.fail_after(50).wait }
    end
    assert_equal "Something went wrong", err.message
  end

  def test_many_concurrent_sleeps_finish_in_parallel
    started = Time.now
    results = Sync do |task|
      handles = 100.times.map { task.async { MyModule.sleep(100).wait } }
      handles.map(&:wait)
    end
    elapsed = Time.now - started

    assert_equal 100, results.length
    assert(results.all? { |r| r.sleep_time == 100 })
    assert_operator elapsed, :<, 2.0, "100 concurrent 100ms sleeps took #{elapsed}s"
  end

  def test_fd_count_is_stable_across_iterations
    # Re-running sleep() many times must not leak the pipe fd or any of the
    # IO objects the proc creates. Sample /proc/self/fd before and after.
    before = Dir.children("/proc/self/fd").length
    Sync do
      50.times { MyModule.sleep(5).wait }
    end
    GC.start
    after = Dir.children("/proc/self/fd").length

    # Allow a small tolerance for transient fds from the test harness itself.
    assert_operator (after - before).abs, :<=, 4,
      "fd count changed from #{before} to #{after} over 50 iterations"
  end

  def test_error_does_not_leak_fds
    before = Dir.children("/proc/self/fd").length
    Sync do
      50.times do
        begin
          MyModule.fail_after(5).wait
        rescue MyModule::Error
          # expected
        end
      end
    end
    GC.start
    after = Dir.children("/proc/self/fd").length

    assert_operator (after - before).abs, :<=, 4,
      "fd count changed from #{before} to #{after} over 50 error iterations"
  end
end

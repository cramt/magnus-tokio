require_relative "magnus_tokio/magnus_tokio"
require "async"
require "io/stream"
require "async/scheduler"

scheduler = Async::Scheduler.new
Fiber.set_scheduler(scheduler)

module Tokio
  def async_wrap_io(io, task: Async::Task.current)
    task.async do
      IO.Stream(io).read
    end
  end

  module_function :async_wrap_io

  def main
    Async do |parent|
      5.times.map do |_|
        parent.async do
          async_wrap_io(sleep(2000))
        end
      end.map(&:wait)
    end
  end

  module_function :main
end

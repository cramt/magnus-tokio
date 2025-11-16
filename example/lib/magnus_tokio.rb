require_relative "magnus_tokio_example/magnus_tokio_example"
require "async"
require "io/stream"
require "async/scheduler"

Fiber.set_scheduler(Async::Scheduler.new)

module MyModule
  def main
    Async do |parent|
      5.times.map do |_|
        parent.async do
          sleep(2000)
        end
      end.map(&:wait)
    end
    Fiber.scheduler.run
  end

  module_function :main
end

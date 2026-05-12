require_relative "magnus_tokio_example/magnus_tokio_example"
require "async"

module MyModule
  def main
    Sync do |task|
      sleepers = 5.times.map { task.async { MyModule.sleep(2000).wait } }
      sleepers.each(&:wait)

      begin
        MyModule.fail_after(1000).wait
      rescue MyModule::Error => e
        puts "Caught expected error: #{e.message}"
        puts "Error class: #{e.class}"
      end
    end
  end

  module_function :main
end

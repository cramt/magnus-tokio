require_relative "magnus_tokio/magnus_tokio"
require "async"
require "io/stream"

module MyThing
  def try_it
    Async do
      fd = some_async_thing
      io = IO.for_fd(fd, autoclose: true)
      io.binmode

      stream = IO::Stream(io)

      while (chunk = stream.read_partial)
        print "ruby got: #{chunk}"
      end

      puts "ruby: EOF"
    end
  end

  module_function :try_it
end

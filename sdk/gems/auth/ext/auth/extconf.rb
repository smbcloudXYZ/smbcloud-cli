# frozen_string_literal: true

require 'mkmf'
require 'rb_sys/mkmf'

create_rust_makefile('auth/auth')

# rb-sys (fixup_libnames) appends `install_name_tool -id "" $(DLLIB)` to the
# generated Makefile, blanking the bundle's LC_ID_DYLIB install name. dyld on
# macOS 26/27 rejects an empty dylib-ID string ("load command string extends
# beyond end of load command"), so the extension fails to dlopen. Rewrite that
# step to stamp a valid @rpath id instead. This covers the `gem install`
# (extconf) build path; the Rakefile covers `rake native gem` builds.
if RUBY_PLATFORM.include?('darwin') && File.exist?('Makefile')
  makefile = File.read('Makefile')
  patched  = makefile.gsub('install_name_tool -id ""', 'install_name_tool -id "@rpath/auth.bundle"')
  File.write('Makefile', patched) if patched != makefile
end

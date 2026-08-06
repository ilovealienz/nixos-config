-- entry point. monitors must load first so rules/binds apply on top
-- of the correct display configuration.

require("modules/monitors")
require("modules/appearance")
require("modules/input")
require("modules/rules")
require("modules/binds")
require("modules/autostart")

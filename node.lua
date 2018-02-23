node.alias("quantum-werewolf")

gl.setup(NATIVE_WIDTH, NATIVE_HEIGHT)

local json = require "json"
local text = require "text"

local write = text{font=dejavu_sans, width=WIDTH, height=HEIGHT, r=1, g=1, b=1}

local data = nil
util.file_watch("data.json", function(content)
    data = json.decode(content)
end)

function node.render()
    gl.clear(0, 0, 0, 1)

    if data == nil or data.mode == nil or data.mode == json.null then
        gl.clear(1, 0, 0, 1)
        write{text={{"?"}}, size=200}
        return
    end
    if data.mode == "error" then
        gl.clear(1, 0, 0, 1)
        if data.error == nil or data.error == json.null then
            write{text={{"error"}}, size=200}
        elseif text_height{text=data.error, size=24} > HEIGHT then
            write{text=data.error, size=12, halign="left"}
        elseif text_height{text=data.error, size=50} > HEIGHT then
            write{text=data.error, size=24, halign="left"}
        else
            write{text=data.error, size=50, halign="left"}
        end
    else
        gl.clear(1, 0, 0, 1)
        write{text={{"unknown", "mode:", data.mode}}}
    end
end

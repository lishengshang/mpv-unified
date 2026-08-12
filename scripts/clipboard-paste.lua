-- clipboard-paste.lua
-- 从系统剪贴板读取 URL 并播放
-- 解决 mpv Wayland 剪贴板属性返回空的问题

local mp = require 'mp'
local utils = require 'mp.utils'

local function paste_and_play()
    -- Try wl-paste (Wayland) then xclip (X11)
    local args = {'sh', '-c', 'wl-paste -n 2>/dev/null || xclip -selection clipboard -o 2>/dev/null'}
    local res = mp.command_native({
        name = 'subprocess',
        playback_only = false,
        capture_stdout = true,
        args = args,
    })
    if res.status == 0 and res.stdout and res.stdout ~= '' then
        local text = res.stdout:match('^(.-)%s*$') -- trim
        if text ~= '' then
            mp.commandv('loadfile', text)
            return
        end
    end
    mp.osd_message('剪贴板为空或无法读取')
end

-- 仅注册绑定名，按键由 input.conf 中 `script-binding clipboard-paste` 绑定
mp.add_key_binding(nil, 'clipboard-paste', paste_and_play)

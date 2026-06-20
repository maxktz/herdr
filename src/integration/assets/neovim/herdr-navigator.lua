-- HERDR_INTEGRATION_VERSION=6

local function is_herdr_pane()
  return vim.env.HERDR_ENV == "1"
    or (vim.env.HERDR_SOCKET_PATH and vim.env.HERDR_SOCKET_PATH ~= "")
    or (vim.env.HERDR_PANE_ID and vim.env.HERDR_PANE_ID ~= "")
end

if not is_herdr_pane() then
  return
end

if vim.g.loaded_herdr_navigator == 1 then
  return
end
vim.g.loaded_herdr_navigator = 1

local M = {}

local directions = {
  left = { wincmd = "h", keys = { "<C-h>", "<BS>" }, command = "HerdrNavigateLeft" },
  down = { wincmd = "j", keys = { "<C-j>", "<NL>" }, command = "HerdrNavigateDown" },
  up = { wincmd = "k", keys = { "<C-k>" }, command = "HerdrNavigateUp" },
  right = { wincmd = "l", keys = { "<C-l>" }, command = "HerdrNavigateRight" },
}

M.last_command = nil
M.last_navigation = nil
M.last_result = nil

local function command_label(args)
  return table.concat(vim.tbl_map(vim.fn.shellescape, args), " ")
end

local function record_result(args, code, stdout, stderr)
  M.last_command = command_label(args)
  M.last_result = {
    code = code,
    stdout = stdout or "",
    stderr = stderr or "",
  }

  if code ~= 0 then
    local message = stderr and stderr ~= "" and stderr or stdout
    vim.notify(
      "Herdr navigation failed: " .. (message or "exit " .. tostring(code)),
      vim.log.levels.WARN
    )
  end
end

local function spawn(args)
  M.last_command = command_label(args)
  M.last_result = nil

  if vim.system then
    vim.system(args, { text = true }, function(result)
      vim.schedule(function()
        record_result(args, result.code, result.stdout, result.stderr)
      end)
    end)
  else
    local stdout = {}
    local stderr = {}
    vim.fn.jobstart(args, {
      stdout_buffered = true,
      stderr_buffered = true,
      on_stdout = function(_, data)
        stdout = data or {}
      end,
      on_stderr = function(_, data)
        stderr = data or {}
      end,
      on_exit = function(_, code)
        record_result(args, code, table.concat(stdout, "\n"), table.concat(stderr, "\n"))
      end,
    })
  end
end

local function herdr_args(direction)
  local bin = vim.env.HERDR_BIN_PATH or "herdr"
  local args = { bin, "pane", "focus", "--direction", direction }
  if vim.env.HERDR_PANE_ID and vim.env.HERDR_PANE_ID ~= "" then
    vim.list_extend(args, { "--pane", vim.env.HERDR_PANE_ID })
  else
    vim.list_extend(args, { "--current" })
  end
  return args
end

function M.navigate(direction)
  local spec = directions[direction]
  if not spec then
    vim.notify("unknown Herdr navigation direction: " .. tostring(direction), vim.log.levels.ERROR)
    return
  end

  local before = vim.api.nvim_get_current_win()
  M.last_navigation = {
    direction = direction,
    mode = vim.api.nvim_get_mode().mode,
    before_win = before,
    after_win = nil,
    moved_in_neovim = false,
    spawned_herdr = false,
  }

  vim.cmd("wincmd " .. spec.wincmd)
  local after = vim.api.nvim_get_current_win()
  M.last_navigation.after_win = after
  if after ~= before then
    M.last_navigation.moved_in_neovim = true
    return
  end

  M.last_navigation.spawned_herdr = true
  spawn(herdr_args(direction))
end

function M.setup(opts)
  opts = opts or {}
  if opts.mappings == false or vim.g.herdr_navigator_no_mappings == 1 then
    return
  end

  local function apply_mappings()
    if vim.g.herdr_navigator_no_mappings == 1 then
      return
    end

    for direction, spec in pairs(directions) do
      for _, key in ipairs(spec.keys) do
        vim.keymap.set("n", key, function()
          M.navigate(direction)
        end, { silent = true, desc = "Herdr navigate " .. direction })
        vim.keymap.set("t", key, function()
          return vim.bo.filetype == "fzf" and key
            or [[<C-\><C-n><cmd>]] .. spec.command .. [[<CR>]]
        end, { expr = true, silent = true, desc = "Herdr navigate " .. direction })
      end
    end
  end

  M.apply_mappings = apply_mappings
  apply_mappings()
  vim.schedule(apply_mappings)
  vim.api.nvim_create_autocmd("VimEnter", {
    callback = function()
      apply_mappings()
      vim.defer_fn(apply_mappings, 100)
    end,
  })
  vim.api.nvim_create_autocmd("User", {
    pattern = { "LazyDone", "VeryLazy" },
    callback = apply_mappings,
  })

end

vim.api.nvim_create_user_command("HerdrNavigateLeft", function()
  M.navigate("left")
end, {})
vim.api.nvim_create_user_command("HerdrNavigateDown", function()
  M.navigate("down")
end, {})
vim.api.nvim_create_user_command("HerdrNavigateUp", function()
  M.navigate("up")
end, {})
vim.api.nvim_create_user_command("HerdrNavigateRight", function()
  M.navigate("right")
end, {})
vim.api.nvim_create_user_command("HerdrNavigatorDebug", function()
  local lines = {
    "HERDR_PANE_ID=" .. (vim.env.HERDR_PANE_ID or ""),
    "HERDR_SOCKET_PATH=" .. (vim.env.HERDR_SOCKET_PATH or ""),
    "HERDR_BIN_PATH=" .. (vim.env.HERDR_BIN_PATH or ""),
    "last_command=" .. (M.last_command or ""),
  }
  if M.last_navigation then
    table.insert(lines, "last_direction=" .. tostring(M.last_navigation.direction))
    table.insert(lines, "last_mode=" .. tostring(M.last_navigation.mode))
    table.insert(lines, "last_before_win=" .. tostring(M.last_navigation.before_win))
    table.insert(lines, "last_after_win=" .. tostring(M.last_navigation.after_win))
    table.insert(lines, "last_moved_in_neovim=" .. tostring(M.last_navigation.moved_in_neovim))
    table.insert(lines, "last_spawned_herdr=" .. tostring(M.last_navigation.spawned_herdr))
  end
  if M.last_result then
    table.insert(lines, "last_exit=" .. tostring(M.last_result.code))
    table.insert(lines, "last_stdout=" .. M.last_result.stdout)
    table.insert(lines, "last_stderr=" .. M.last_result.stderr)
  end
  vim.notify(table.concat(lines, "\n"), vim.log.levels.INFO)
end, {})

M.setup()

return M

if script ~= nil then
	local parent = script.Parent
	if parent ~= nil then
		local module = parent:FindFirstChild("MathModule")
		if module ~= nil then
			local MathModule = require(module)
			print("Main loaded, 1 + 2 = " .. tostring(MathModule.add(1, 2)))
		end
	end
end

print("Main example loaded")

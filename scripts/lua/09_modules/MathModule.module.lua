local MathModule = {}

local module_counter = 0

function MathModule.add(a, b)
	module_counter = module_counter + 1
	return a + b
end

function MathModule.double(x)
	return x * 2
end

function MathModule.lerp(a, b, t)
	return a + (b - a) * t
end

return MathModule

#	Simple return
def simple(x):
	return "Function returns: '" + x + "'"

print(simple("Sent to function"))
#	Output: Function returns: 'Sent to function'

#	Return multiple
def multi_return():
	str = "Something"
	integer = 6
	return str, integer

print(multi_return())
#	Output: ('Something', 6)

for i in range(1, 100):
	x = ""
	if i % 3 == 0:
		x += "Fizz"
	if i % 5 == 0:
		x += "Buzz"
	if i % 3 != 0 and i % 5 != 0:
		x += str(i)
	print(x)

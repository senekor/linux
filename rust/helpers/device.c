// SPDX-License-Identifier: GPL-2.0

#include <linux/device.h>
#include <linux/property.h>

int rust_helper_devm_add_action(struct device *dev,
				void (*action)(void *),
				void *data)
{
	return devm_add_action(dev, action, data);
}

struct fwnode_handle *rust_helper_dev_fwnode(struct device *dev)
{
	return dev_fwnode(dev);
}

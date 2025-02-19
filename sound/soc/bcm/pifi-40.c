// SPDX-License-Identifier: GPL-2.0-only
/*
 * ALSA ASoC Machine Driver for PiFi-40
 *
 * Author:	David Knell <david.knell@gmail.com)
 *		based on code by Daniel Matuschek <info@crazy-audio.com>
 *		based on code by Florian Meier <florian.meier@koalo.de>
 * Copyright (C) 2020
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * version 2 as published by the Free Software Foundation.
 *
 * This program is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License for more details.
 */

#include <linux/module.h>
#include <linux/platform_device.h>
#include <linux/gpio/consumer.h>
#include <sound/core.h>
#include <sound/pcm.h>
#include <sound/pcm_params.h>
#include <sound/soc.h>
#include <linux/firmware.h>
#include <linux/delay.h>
#include <sound/tlv.h>

static struct gpio_desc *pdn_gpio;
static int vol = 0x30;

// Volume control
static int pifi_40_vol_get(struct snd_kcontrol *kcontrol,
			   struct snd_ctl_elem_value *ucontrol)
{
	ucontrol->value.integer.value[0] = vol;
	ucontrol->value.integer.value[1] = vol;
	return 0;
}

static int pifi_40_vol_set(struct snd_kcontrol *kcontrol,
			   struct snd_ctl_elem_value *ucontrol)
{
	struct snd_soc_card *card = snd_kcontrol_chip(kcontrol);
	struct snd_soc_pcm_runtime *rtd;
	unsigned int v = ucontrol->value.integer.value[0];
	struct snd_soc_component *dac[2];

	rtd = snd_soc_get_pcm_runtime(card, &card->dai_link[0]);
	dac[0] = snd_soc_rtd_to_codec(rtd, 0)->component;
	dac[1] = snd_soc_rtd_to_codec(rtd, 1)->component;

	snd_soc_component_write(dac[0], 0x07, 255 - v);
	snd_soc_component_write(dac[1], 0x07, 255 - v);

	vol = v;
	return 1;
}

static const DECLARE_TLV_DB_SCALE(digital_tlv_master, -10350, 50, 1);
static const struct snd_kcontrol_new pifi_40_controls[] = {
	SOC_DOUBLE_R_EXT_TLV("Master Volume", 0x00, 0x01,
			     0x00, // Min
			     0xff, // Max
			     0x01, // Invert
			     pifi_40_vol_get, pifi_40_vol_set,
			     digital_tlv_master)
};

static const char * const codec_ctl_pfx[] = { "Left", "Right" };

static const char * const codec_ctl_name[] = { "Master Volume",
					"Speaker Volume",
					"Speaker Switch" };

static int snd_pifi_40_init(struct snd_soc_pcm_runtime *rtd)
{
	struct snd_soc_card *card = rtd->card;
	struct snd_soc_component *dac[2];
	struct snd_kcontrol *kctl;
	int i, j;

	dac[0] = snd_soc_rtd_to_codec(rtd, 0)->component;
	dac[1] = snd_soc_rtd_to_codec(rtd, 1)->component;


	// Set up cards - pulse power down first
	gpiod_set_value_cansleep(pdn_gpio, 1);
	usleep_range(1000, 10000);
	gpiod_set_value_cansleep(pdn_gpio, 0);
	usleep_range(20000, 30000);

	// Oscillator trim
	snd_soc_component_write(dac[0], 0x1b, 0);
	snd_soc_component_write(dac[1], 0x1b, 0);
	usleep_range(60000, 80000);

	// Common setup
	for (i = 0; i < 2; i++) {
		// MCLK at 64fs, sample rate 44.1 or 48kHz
		snd_soc_component_write(dac[i], 0x00, 0x60);

		// Set up for PBTL
		snd_soc_component_write(dac[i], 0x19, 0x3A);
		snd_soc_component_write(dac[i], 0x25, 0x01103245);

		// Master vol to -10db
		snd_soc_component_write(dac[i], 0x07, 0x44);
	}
	// Inputs set to L and R respectively
	snd_soc_component_write(dac[0], 0x20, 0x00017772);
	snd_soc_component_write(dac[1], 0x20, 0x00107772);

	// Remove codec controls
	for (i = 0; i < 2; i++) {
		for (j = 0; j < 3; j++) {
			char cname[256];

			sprintf(cname, "%s %s", codec_ctl_pfx[i],
				codec_ctl_name[j]);
			kctl = snd_soc_card_get_kcontrol(card, cname);
			if (!kctl) {
				pr_info("Control %s not found\n",
				       cname);
			} else {
				kctl->vd[0].access =
					SNDRV_CTL_ELEM_ACCESS_READWRITE;
				snd_ctl_remove(card->snd_card, kctl);
			}
		}
	}

	return 0;
}

static int snd_pifi_40_hw_params(struct snd_pcm_substream *substream,
				 struct snd_pcm_hw_params *params)
{
	struct snd_soc_pcm_runtime *rtd = substream->private_data;
	struct snd_soc_dai *cpu_dai = snd_soc_rtd_to_cpu(rtd, 0);

	return snd_soc_dai_set_bclk_ratio(cpu_dai, 64);
}

static struct snd_soc_ops snd_pifi_40_ops = { .hw_params =
						      snd_pifi_40_hw_params };

static struct snd_soc_dai_link_component pifi_40_codecs[] = {
	{
		.dai_name = "tas571x-hifi",
	},
	{
		.dai_name = "tas571x-hifi",
	},
};

SND_SOC_DAILINK_DEFS(
	pifi_40_dai, DAILINK_COMP_ARRAY(COMP_EMPTY()),
	DAILINK_COMP_ARRAY(COMP_CODEC("tas571x.1-001a", "tas571x-hifi"),
			   COMP_CODEC("tas571x.1-001b", "tas571x-hifi")),
	DAILINK_COMP_ARRAY(COMP_EMPTY()));

static struct snd_soc_dai_link snd_pifi_40_dai[] = {
	{
		.name = "PiFi40",
		.stream_name = "PiFi40",
		.dai_fmt = SND_SOC_DAIFMT_I2S | SND_SOC_DAIFMT_NB_NF |
			   SND_SOC_DAIFMT_CBS_CFS,
		.ops = &snd_pifi_40_ops,
		.init = snd_pifi_40_init,
		SND_SOC_DAILINK_REG(pifi_40_dai),
	},
};

// Machine driver
static struct snd_soc_card snd_pifi_40 = {
	.name = "PiFi40",
	.owner = THIS_MODULE,
	.dai_link = snd_pifi_40_dai,
	.num_links = ARRAY_SIZE(snd_pifi_40_dai),
	.controls = pifi_40_controls,
	.num_controls = ARRAY_SIZE(pifi_40_controls)
};

static void snd_pifi_40_pdn(struct snd_soc_card *card, int on)
{
	if (pdn_gpio)
		gpiod_set_value_cansleep(pdn_gpio, on ? 0 : 1);
}

static int snd_pifi_40_probe(struct platform_device *pdev)
{
	struct snd_soc_card *card = &snd_pifi_40;
	int ret = 0, i = 0;

	card->dev = &pdev->dev;
	platform_set_drvdata(pdev, &snd_pifi_40);

	if (pdev->dev.of_node) {
		struct device_node *i2s_node;
		struct snd_soc_dai_link *dai;

		dai = &snd_pifi_40_dai[0];
		i2s_node = of_parse_phandle(pdev->dev.of_node, "i2s-controller",
					    0);
		if (i2s_node) {
			for (i = 0; i < card->num_links; i++) {
				dai->cpus->dai_name = NULL;
				dai->cpus->of_node = i2s_node;
				dai->platforms->name = NULL;
				dai->platforms->of_node = i2s_node;
			}
		}

		pifi_40_codecs[0].of_node =
			of_parse_phandle(pdev->dev.of_node, "audio-codec", 0);
		pifi_40_codecs[1].of_node =
			of_parse_phandle(pdev->dev.of_node, "audio-codec", 1);
		if (!pifi_40_codecs[0].of_node || !pifi_40_codecs[1].of_node) {
			dev_err(&pdev->dev,
				"Property 'audio-codec' missing or invalid\n");
			return -EINVAL;
		}

		pdn_gpio = devm_gpiod_get_optional(&pdev->dev, "pdn",
						   GPIOD_OUT_LOW);
		if (IS_ERR(pdn_gpio)) {
			ret = PTR_ERR(pdn_gpio);
			dev_err(&pdev->dev, "failed to get pdn gpio: %d\n",
				ret);
			return ret;
		}

		ret = snd_soc_register_card(&snd_pifi_40);
		if (ret < 0) {
			dev_err(&pdev->dev,
				"snd_soc_register_card() failed: %d\n", ret);
			return ret;
		}

		return 0;
	}

	return -EINVAL;
}

static void snd_pifi_40_remove(struct platform_device *pdev)
{
	struct snd_soc_card *card = platform_get_drvdata(pdev);

	kfree(&card->drvdata);
	snd_pifi_40_pdn(&snd_pifi_40, 0);
	snd_soc_unregister_card(&snd_pifi_40);
}

static const struct of_device_id snd_pifi_40_of_match[] = {
	{
		.compatible = "pifi,pifi-40",
	},
	{ /* sentinel */ },
};

MODULE_DEVICE_TABLE(of, snd_pifi_40_of_match);

static struct platform_driver snd_pifi_40_driver = {
	.driver = {
		.name = "snd-pifi-40",
		.owner = THIS_MODULE,
		.of_match_table = snd_pifi_40_of_match,
	},
	.probe = snd_pifi_40_probe,
	.remove = snd_pifi_40_remove,
};

module_platform_driver(snd_pifi_40_driver);

MODULE_AUTHOR("David Knell <david.knell@gmail.com>");
MODULE_DESCRIPTION("ALSA ASoC Machine Driver for PiFi-40");
MODULE_LICENSE("GPL v2");

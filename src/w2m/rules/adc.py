"""ADC (Intel SDM Vol 2, ADC)."""

from w2m.rules import _int

ICLASSES = ('ADC',)
SHARED = ("_int",)
rule = _int.adc

import unittest
from fractions import Fraction
from pathlib import Path

from gacha_engine import AtLeastRarity, Engine


ROOT = Path(__file__).resolve().parents[3]
POSITIVE = ROOT / "spec" / "gacharule" / "positive"
ORDER = [
    "endfield.gmr",
    "genshin.gmr",
    "weapon_ten.gmr",
    "sample.gml",
    "banners.gmb",
]


def rarity(draw, grade):
    total = Fraction(0)
    for branch in draw.branches:
        if int(branch.rarity) == grade:
            total += Fraction(int(branch.numer), int(branch.denom))
    return total


class ContractTest(unittest.TestCase):
    def test_positive_rates_and_analysis(self):
        sources = [
            {"name": name, "text": (POSITIVE / name).read_text(encoding="utf-8")}
            for name in ORDER
        ]
        engine = Engine.compile(sources)
        session = engine.session("endfield_event_a")
        session.set_count("pity6", "65")
        draw = session.draw("draw")
        self.assertEqual(rarity(draw, 6), Fraction(29, 500))
        report = session.analyze(
            "draw",
            times=1,
            actions=1,
            copies=1,
            target=AtLeastRarity("6"),
            quantiles=[29 / 500],
        )
        self.assertAlmostEqual(report.probability_within, 29 / 500, places=9)
        self.assertEqual(report.draw_quantiles[0].has_index, 1)
        self.assertEqual(report.draw_quantiles[0].index, 1)

        genshin = engine.session("genshin_event_a").draw("draw")
        self.assertEqual(rarity(genshin, 5), Fraction(3, 500))
        self.assertEqual(rarity(genshin, 4), Fraction(51, 1000))
        base = engine.session("genshin_event_a").analyze(
            "draw",
            times=1,
            actions=1,
            target=AtLeastRarity(5),
        )
        self.assertAlmostEqual(base.probability_within, 3 / 500, places=9)


if __name__ == "__main__":
    unittest.main()

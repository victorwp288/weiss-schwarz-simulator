window.BENCHMARK_DATA = {
  "lastUpdate": 1767567647096,
  "repoUrl": "https://github.com/victorwp288/weiss-schwarz-simulator",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "committer": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "distinct": true,
          "id": "2126a9ebbf1750e3dc557094865e4d15d0b66842",
          "message": "ci: fix wheels/bench triggers + sccache",
          "timestamp": "2026-01-04T22:54:16+01:00",
          "tree_id": "8687e76a08bd187c9dbb7b2be396c1c0f4326370",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/2126a9ebbf1750e3dc557094865e4d15d0b66842"
        },
        "date": 1767563955590,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 63182,
            "range": "± 199",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26470,
            "range": "± 167",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 115638,
            "range": "± 461",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 109410,
            "range": "± 252",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1483,
            "range": "± 1750",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1485,
            "range": "± 1732",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 221,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 228,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 421,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 427,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 186,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 884,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 199,
            "range": "± 5",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "committer": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "distinct": true,
          "id": "2e5431702e2f19b5edcc9439a18a33a20ebb6411",
          "message": "ci: improve benchmark page visibility",
          "timestamp": "2026-01-04T23:21:58+01:00",
          "tree_id": "31b26ad343ca54cfc6fbd53932e146c23b53b8d0",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/2e5431702e2f19b5edcc9439a18a33a20ebb6411"
        },
        "date": 1767565570128,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 62962,
            "range": "± 3564",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26760,
            "range": "± 1089",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 116317,
            "range": "± 796",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 108177,
            "range": "± 3120",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1530,
            "range": "± 9334",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1482,
            "range": "± 502",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 221,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 227,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 412,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 409,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 185,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 156,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 873,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 179,
            "range": "± 1",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "committer": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "distinct": true,
          "id": "f16e6563a6e1df2305296f87fb63a75186126b33",
          "message": "ci: fix benchmark page patch step",
          "timestamp": "2026-01-04T23:27:57+01:00",
          "tree_id": "0e11777aa0c72acec322f053f32f0478b65a4ef6",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/f16e6563a6e1df2305296f87fb63a75186126b33"
        },
        "date": 1767565927886,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 63288,
            "range": "± 451",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26336,
            "range": "± 679",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 121032,
            "range": "± 1693",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 117206,
            "range": "± 2698",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1546,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1536,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 221,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 227,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 418,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 422,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 190,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 880,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 198,
            "range": "± 4",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "committer": {
            "email": "victorwpetersen@gmail.com",
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288"
          },
          "distinct": true,
          "id": "0b3faaa1330b4cd403ec574c52e5b56908c2138a",
          "message": "docs: pin security badge to main",
          "timestamp": "2026-01-04T23:55:26+01:00",
          "tree_id": "cea11d27229964a7cb02378240087957b7c4530e",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/0b3faaa1330b4cd403ec574c52e5b56908c2138a"
        },
        "date": 1767567646240,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 63281,
            "range": "± 471",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26500,
            "range": "± 895",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 117553,
            "range": "± 572",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 111286,
            "range": "± 789",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1510,
            "range": "± 4473",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1513,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 206,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 211,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 410,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 432,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 186,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 158,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 867,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 197,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}
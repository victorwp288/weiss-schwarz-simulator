window.BENCHMARK_DATA = {
  "lastUpdate": 1767565570987,
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
      }
    ]
  }
}
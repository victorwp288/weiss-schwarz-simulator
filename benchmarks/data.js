window.BENCHMARK_DATA = {
  "lastUpdate": 1771410891805,
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
          "id": "5c3cacfda4b899aefb8decae0d5fdca63e11b3ff",
          "message": "fix: align package versions with v0.1.1",
          "timestamp": "2026-01-05T00:49:17+01:00",
          "tree_id": "66ccca05177d0c8a95ff3c41a4cde4686e88ed7d",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/5c3cacfda4b899aefb8decae0d5fdca63e11b3ff"
        },
        "date": 1767570814404,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 61348,
            "range": "± 943",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 23576,
            "range": "± 217",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 135894,
            "range": "± 336",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 136536,
            "range": "± 520",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1550,
            "range": "± 3359",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1541,
            "range": "± 249",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 213,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 210,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 397,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 404,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 142,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 59,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 133,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 1058,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 133,
            "range": "± 2",
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
          "id": "f3425054abe5ba3ad08853b2b53e1d5930b91b10",
          "message": "ci: build wheel for pytest",
          "timestamp": "2026-01-05T00:58:39+01:00",
          "tree_id": "dbbe1fa276aa2bc42d0a09af391e52bef15897ba",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/f3425054abe5ba3ad08853b2b53e1d5930b91b10"
        },
        "date": 1767571368965,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 63802,
            "range": "± 1246",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 23455,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 136761,
            "range": "± 734",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 137390,
            "range": "± 760",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1607,
            "range": "± 959",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1658,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 216,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 215,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 398,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 398,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 140,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 59,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 132,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 1051,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 132,
            "range": "± 0",
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
          "id": "cc0bc74b1317b1164433ebb2597c03efd93c5df9",
          "message": "test: remove invariants doc check",
          "timestamp": "2026-01-05T01:10:21+01:00",
          "tree_id": "4f4e9828b51e67068dc1b5ebcbfbdaa88f656ba3",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/cc0bc74b1317b1164433ebb2597c03efd93c5df9"
        },
        "date": 1767572063162,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 60197,
            "range": "± 1092",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 23093,
            "range": "± 350",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 106738,
            "range": "± 530",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 106355,
            "range": "± 587",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1526,
            "range": "± 146",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1485,
            "range": "± 1235",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 225,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 224,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 444,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 445,
            "range": "± 1",
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
            "value": 50,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 152,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 899,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 180,
            "range": "± 0",
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
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0063f67fed53ffdf53a2987fecfe444e669f6d18",
          "message": "Merge pull request #2 from victorwp288/release-please--branches--main\n\nchore(main): release 0.1.2",
          "timestamp": "2026-01-05T01:16:34+01:00",
          "tree_id": "0a57309429aa55f6b89fe9e694a97f1d55b2e3b5",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/0063f67fed53ffdf53a2987fecfe444e669f6d18"
        },
        "date": 1767572437041,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 62940,
            "range": "± 4345",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26161,
            "range": "± 430",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 109888,
            "range": "± 957",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 108526,
            "range": "± 531",
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
            "value": 1488,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1478,
            "range": "± 44",
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
            "value": 226,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 412,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 419,
            "range": "± 2",
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
            "value": 157,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 893,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 178,
            "range": "± 8",
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
          "id": "079e9e086e5ea46371d2e1376458f5d9cdd04ba8",
          "message": "chore: add audit hooks to pre-commit",
          "timestamp": "2026-01-05T01:39:52+01:00",
          "tree_id": "5213e5a1ce941423984109c9e16f096364b2270d",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/079e9e086e5ea46371d2e1376458f5d9cdd04ba8"
        },
        "date": 1767573852810,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 63280,
            "range": "± 3151",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26268,
            "range": "± 223",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 111592,
            "range": "± 700",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 109646,
            "range": "± 523",
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
            "value": 43,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1534,
            "range": "± 4537",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1540,
            "range": "± 4480",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 228,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 233,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 455,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 412,
            "range": "± 1",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 158,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 859,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 199,
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
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dd1008f7ccbde231cc639a03d3fef57f1f3cbbe5",
          "message": "Merge pull request #3 from victorwp288/release-please--branches--main\n\nchore(main): release 0.1.3",
          "timestamp": "2026-01-05T01:55:57+01:00",
          "tree_id": "698e1c8a2b13e74e34ad1fa1743c0fb9d0f6509d",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/dd1008f7ccbde231cc639a03d3fef57f1f3cbbe5"
        },
        "date": 1767574809371,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 63126,
            "range": "± 2283",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 26968,
            "range": "± 211",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 117321,
            "range": "± 1028",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 117409,
            "range": "± 875",
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
            "value": 1487,
            "range": "± 6924",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1490,
            "range": "± 10229",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 227,
            "range": "± 1",
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
            "value": 457,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 410,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 188,
            "range": "± 4",
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
            "value": 157,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 874,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 185,
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
          "id": "6f69d20ae969faf425d29685429ff1d6119addb6",
          "message": "Format weiss_sim init",
          "timestamp": "2026-02-04T09:37:53+01:00",
          "tree_id": "91c28b6e20f0bee04cae5e929ed0bee5c56c3d68",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/6f69d20ae969faf425d29685429ff1d6119addb6"
        },
        "date": 1770194605513,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 32840,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 14661,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 67545,
            "range": "± 619",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 67013,
            "range": "± 402",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1021,
            "range": "± 6925",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1022,
            "range": "± 13410",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 169,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 172,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 421,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 414,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 174,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 112,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 888,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 185,
            "range": "± 0",
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
          "id": "36d5bbde1586e3fbba2da5363017fdfc14d3d2c1",
          "message": "Pin ruff version for CI formatting",
          "timestamp": "2026-02-04T09:51:18+01:00",
          "tree_id": "506f592e889f8aadc789d4f9a760a2ed3e7b9c4d",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/36d5bbde1586e3fbba2da5363017fdfc14d3d2c1"
        },
        "date": 1770195370349,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 31245,
            "range": "± 2559",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15036,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 74182,
            "range": "± 2898",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 67407,
            "range": "± 1538",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1098,
            "range": "± 6896",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1094,
            "range": "± 6959",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 172,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 171,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 407,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 408,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 175,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 112,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 876,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 184,
            "range": "± 0",
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
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "56a3082e46433cececaafdc7bb077fdba2188cd0",
          "message": "Merge pull request #5 from victorwp288/release-please--branches--main\n\nchore(main): release 0.2.0",
          "timestamp": "2026-02-04T10:11:17+01:00",
          "tree_id": "e9aa74235da72b15c41813f2a761b80cfb866692",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/56a3082e46433cececaafdc7bb077fdba2188cd0"
        },
        "date": 1770196588446,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 31597,
            "range": "± 278",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15154,
            "range": "± 1013",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 79056,
            "range": "± 3513",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 67644,
            "range": "± 349",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1118,
            "range": "± 6906",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1050,
            "range": "± 6760",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 177,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 176,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 403,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 391,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 174,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 110,
            "range": "± 0",
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
            "value": 185,
            "range": "± 0",
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
          "id": "5f7507540d729b168394b401d809d5bf399ccb53",
          "message": "Include license files in sdist",
          "timestamp": "2026-02-04T10:18:24+01:00",
          "tree_id": "298fa567d2667ac7a44ba626230a2346bcda89bc",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/5f7507540d729b168394b401d809d5bf399ccb53"
        },
        "date": 1770197006222,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 29251,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15273,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 67200,
            "range": "± 1929",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 67155,
            "range": "± 154",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1065,
            "range": "± 8077",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1071,
            "range": "± 7782",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 171,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 171,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 411,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 421,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 165,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 53,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 108,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 882,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 166,
            "range": "± 0",
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
          "id": "759a86d5ad5c042da2c26e76fd1af93c355c4a18",
          "message": "Fix benchmark workflow branch switch by using temp outputs",
          "timestamp": "2026-02-07T17:54:45+01:00",
          "tree_id": "5f6f3bcde56655fb0e8b4c4bddc3ea0e998b75a2",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/759a86d5ad5c042da2c26e76fd1af93c355c4a18"
        },
        "date": 1770483583124,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 31802,
            "range": "± 313",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 16834,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 80652,
            "range": "± 1677",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 72821,
            "range": "± 2243",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1177,
            "range": "± 6939",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1187,
            "range": "± 13600",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 178,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 185,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 382,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 387,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 174,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 882,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 175,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288",
            "email": "victorwpetersen@gmail.com"
          },
          "committer": {
            "name": "Victor Wejergang Petersen",
            "username": "victorwp288",
            "email": "victorwpetersen@gmail.com"
          },
          "id": "dc0dccd8e5429d0cbb02b298d081ee27d40f9013",
          "message": "Restore benchmark snapshot markers in README",
          "timestamp": "2026-02-07T17:02:08Z",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/dc0dccd8e5429d0cbb02b298d081ee27d40f9013"
        },
        "date": 1770484173323,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 32104,
            "range": "± 176",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15068,
            "range": "± 259",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 73266,
            "range": "± 667",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 73498,
            "range": "± 600",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1139,
            "range": "± 6215",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1141,
            "range": "± 6415",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 164,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 173,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 384,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 380,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 174,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 891,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 175,
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
          "id": "3c7c253022591efc13ff8c3181e38bad148e40c6",
          "message": "Bump package versions to 0.2.1",
          "timestamp": "2026-02-07T18:13:16+01:00",
          "tree_id": "c3e4aeb1c859326cda30442bfc6be6db5a496ffe",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/3c7c253022591efc13ff8c3181e38bad148e40c6"
        },
        "date": 1770484730282,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 31792,
            "range": "± 193",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 16628,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 67375,
            "range": "± 476",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 67209,
            "range": "± 397",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1104,
            "range": "± 2211",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1166,
            "range": "± 6969",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 166,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 166,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 388,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 390,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 175,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 887,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 175,
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
          "id": "053a086f538b4c71cbe44dae8031ec14f26abcca",
          "message": "ci/docs: enforce perf budgets and refresh architecture docs",
          "timestamp": "2026-02-16T00:43:11+01:00",
          "tree_id": "0b9704ecd3235d5084cf7206ba4bb0f367d8f2bb",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/053a086f538b4c71cbe44dae8031ec14f26abcca"
        },
        "date": 1771199398813,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 35275,
            "range": "± 140",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 16965,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "reset_batch_256",
            "value": 898737,
            "range": "± 11571",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 73667,
            "range": "± 364",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 73837,
            "range": "± 266",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1170,
            "range": "± 8421",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1172,
            "range": "± 7725",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 187,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 186,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 392,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 398,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 170,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 891,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 175,
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
          "id": "576da0bf97eaac49a350de6e7d81a6090c9f2044",
          "message": "fix(ci): resolve clippy/ruff failures and perf venv setup",
          "timestamp": "2026-02-16T01:01:01+01:00",
          "tree_id": "3a81304737cee1c8660352f6689f5d4e57f57bea",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/576da0bf97eaac49a350de6e7d81a6090c9f2044"
        },
        "date": 1771200421190,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 33586,
            "range": "± 166",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15533,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "reset_batch_256",
            "value": 920526,
            "range": "± 18272",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 85785,
            "range": "± 247",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 85340,
            "range": "± 146",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1316,
            "range": "± 4042",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1319,
            "range": "± 3520",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 180,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 179,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 373,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 371,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 126,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 114,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 1060,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 134,
            "range": "± 0",
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
          "id": "40e5dc962ca7c6b5ec3309a09dd7dc0259d40884",
          "message": "fix(perf-ci): same-runner perf gating and hot-path optimizations",
          "timestamp": "2026-02-16T09:21:35+01:00",
          "tree_id": "0d61ccdc06086253cc6ea7d3cb3ede553d163a2f",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/40e5dc962ca7c6b5ec3309a09dd7dc0259d40884"
        },
        "date": 1771231000089,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 35677,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15265,
            "range": "± 778",
            "unit": "ns/iter"
          },
          {
            "name": "reset_batch_256",
            "value": 867903,
            "range": "± 7690",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 74124,
            "range": "± 2634",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 80038,
            "range": "± 3066",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1162,
            "range": "± 12274",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1169,
            "range": "± 6396",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 178,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 183,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 394,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 391,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 173,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 887,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 175,
            "range": "± 0",
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
          "id": "7d6ce0f4229d4f2c52e18304c2d248154a5ab62a",
          "message": "Refresh documentation for updated engine and API behavior",
          "timestamp": "2026-02-18T11:29:07+01:00",
          "tree_id": "7126ef54892512da3d84582bd07ed140cd68ac28",
          "url": "https://github.com/victorwp288/weiss-schwarz-simulator/commit/7d6ce0f4229d4f2c52e18304c2d248154a5ab62a"
        },
        "date": 1771410890831,
        "tool": "cargo",
        "benches": [
          {
            "name": "advance_until_decision",
            "value": 51535,
            "range": "± 267",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_64",
            "value": 15387,
            "range": "± 756",
            "unit": "ns/iter"
          },
          {
            "name": "reset_batch_256",
            "value": 879891,
            "range": "± 7521",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_off",
            "value": 80485,
            "range": "± 610",
            "unit": "ns/iter"
          },
          {
            "name": "step_batch_fast_256_priority_on",
            "value": 74685,
            "range": "± 232",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "legal_actions_forced",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_on",
            "value": 1225,
            "range": "± 8226",
            "unit": "ns/iter"
          },
          {
            "name": "on_reverse_decision_frequency_off",
            "value": 1224,
            "range": "± 8382",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode",
            "value": 177,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "observation_encode_forced",
            "value": 180,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction",
            "value": 394,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_construction_forced",
            "value": 400,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "choice_paging_worst_case_mask",
            "value": 179,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_legal_actions",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_observation_encode",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_action_masks_batch_into",
            "value": 874,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_choice_paging_worst_case",
            "value": 189,
            "range": "± 1",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}
from __future__ import annotations

import hashlib
import os

from weiss_sim import catalog as catalog_mod


def test_compute_db_sha256_recomputes_when_file_content_changes(tmp_path):
    db_path = tmp_path / "cards.wsdb"
    db_path.write_bytes(b"a" * 64)
    first_stat = db_path.stat()
    first_hash = catalog_mod.compute_db_sha256(db_path)

    db_path.write_bytes(b"b" * 64)
    os.utime(db_path, ns=(first_stat.st_atime_ns, first_stat.st_mtime_ns))

    second_hash = catalog_mod.compute_db_sha256(db_path)
    assert second_hash != first_hash
    assert second_hash == hashlib.sha256(db_path.read_bytes()).hexdigest()

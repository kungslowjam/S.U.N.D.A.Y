"""Data source connectors for Deep Research."""

from sunday.connectors._stubs import (
    Attachment,
    BaseConnector,
    Document,
    SyncStatus,
)
from sunday.connectors.store import KnowledgeStore

__all__ = ["Attachment", "BaseConnector", "Document", "KnowledgeStore", "SyncStatus"]

# Auto-register built-in connectors
import sunday.connectors.obsidian  # noqa: F401

try:
    import sunday.connectors.gmail  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.gmail_imap  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.gdrive  # noqa: F401
except ImportError:
    pass  # httpx may not be installed

try:
    import sunday.connectors.notion  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.granola  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.gcontacts  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.imessage  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.apple_notes  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.apple_music  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.apple_contacts  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.slack_connector  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.outlook  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.gcalendar  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.dropbox  # noqa: F401
except ImportError:
    pass  # httpx may not be installed

try:
    import sunday.connectors.whatsapp  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.oura  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.apple_health  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.strava  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.spotify  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.google_tasks  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.weather  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.github_notifications  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.hackernews  # noqa: F401
except ImportError:
    pass

try:
    import sunday.connectors.news_rss  # noqa: F401
except ImportError:
    pass

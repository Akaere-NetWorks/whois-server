"""
Pixiv API client using pixivpy3 library
"""

import os
from typing import Dict, Any
from pixivpy3 import AppPixivAPI


def _replace_image_url(url: str) -> str:
    """
    Replace Pixiv image URL with proxy URL if proxy is enabled
    
    Args:
        url: Original Pixiv image URL
        
    Returns:
        Proxied URL if PIXIV_PROXY_ENABLED=true, otherwise original URL
    """
    if not url:
        return url
        
    proxy_enabled = os.getenv('PIXIV_PROXY_ENABLED', 'false').lower() == 'true'
    if not proxy_enabled:
        return url
    
    proxy_base = os.getenv('PIXIV_PROXY_BASE_URL', 'http://localhost:8080/pixiv-proxy')
    pixiv_host = 'https://i.pximg.net'
    
    if url.startswith(pixiv_host):
        # 替换 https://i.pximg.net/... 为 {proxy_base}/...
        return url.replace(pixiv_host, proxy_base)
    
    return url


class PixivClient:
    """Pixiv API client wrapper"""
    
    def __init__(self):
        self.api = AppPixivAPI()
        self._authenticated = False
        self._access_token = None
        self._refresh_token = None
        
    def authenticate(self) -> bool:
        """
        Authenticate with Pixiv using refresh token from environment variable
        Returns True if authentication successful
        """
        if self._authenticated:
            return True
            
        refresh_token = os.getenv('PIXIV_REFRESH_TOKEN')
        if not refresh_token:
            return False
            
        try:
            # 使用 auth() 方法获取 access_token
            result = self.api.auth(refresh_token=refresh_token)
            if result and hasattr(result, 'access_token'):
                self._access_token = result.access_token
                self._refresh_token = result.refresh_token
                # 使用 set_auth() 设置认证信息
                self.api.set_auth(self._access_token, self._refresh_token)
                self._authenticated = True
                return True
            return False
        except Exception as e:
            print(f"Authentication failed: {e}")
            return False
    
    def _ensure_auth(self) -> bool:
        """Ensure client is authenticated before making requests"""
        if not self._authenticated:
            return self.authenticate()
        return True


# Global client instance
_client = PixivClient()


def get_artwork_info(artwork_id: int) -> Dict[str, Any]:
    """
    Get detailed information about a specific artwork
    
    Args:
        artwork_id: Pixiv artwork ID (illust_id)
        
    Returns:
        Dictionary containing artwork information
    """
    if not _client._ensure_auth():
        return {"error": "Not authenticated. Set PIXIV_REFRESH_TOKEN environment variable."}
    
    try:
        result = _client.api.illust_detail(artwork_id)
        
        # ParsedJson 对象可以像字典或对象一样访问
        if not hasattr(result, 'illust'):
            return {"error": "Artwork not found"}
        
        illust = result.illust
        
        return {
            "id": illust.id,
            "title": illust.title,
            "type": illust.type,
            "caption": getattr(illust, 'caption', ''),
            "user": {
                "id": illust.user.id,
                "name": illust.user.name,
                "account": illust.user.account,
            },
            "tags": [tag.name for tag in illust.tags],
            "create_date": illust.create_date,
            "width": illust.width,
            "height": illust.height,
            "page_count": illust.page_count,
            "total_view": illust.total_view,
            "total_bookmarks": illust.total_bookmarks,
            "url": f"https://www.pixiv.net/artworks/{artwork_id}",
            "image_urls": {
                "square_medium": _replace_image_url(getattr(illust.image_urls, 'square_medium', None)),
                "medium": _replace_image_url(getattr(illust.image_urls, 'medium', None)),
                "large": _replace_image_url(getattr(illust.image_urls, 'large', None)),
                "original": _replace_image_url(getattr(illust, 'meta_single_page', {}).get('original_image_url', None)),
            },
            # 多页作品的所有页面
            "meta_pages": [
                {
                    "square_medium": _replace_image_url(page.image_urls.square_medium if hasattr(page.image_urls, 'square_medium') else None),
                    "medium": _replace_image_url(page.image_urls.medium if hasattr(page.image_urls, 'medium') else None),
                    "large": _replace_image_url(page.image_urls.large if hasattr(page.image_urls, 'large') else None),
                    "original": _replace_image_url(page.image_urls.original if hasattr(page.image_urls, 'original') else None),
                }
                for page in getattr(illust, 'meta_pages', [])
            ] if hasattr(illust, 'meta_pages') else [],
        }
    except Exception as e:
        return {"error": str(e)}


def get_user_info(user_id: int) -> Dict[str, Any]:
    """
    Get detailed information about a Pixiv user
    
    Args:
        user_id: Pixiv user ID
        
    Returns:
        Dictionary containing user information
    """
    if not _client._ensure_auth():
        return {"error": "Not authenticated. Set PIXIV_REFRESH_TOKEN environment variable."}
    
    try:
        result = _client.api.user_detail(user_id)
        
        if not hasattr(result, 'user'):
            return {"error": "User not found"}
        
        user = result.user
        profile = result.profile
        
        return {
            "id": user.id,
            "name": user.name,
            "account": user.account,
            "comment": getattr(user, 'comment', ''),
            "profile_image": {
                "medium": user.profile_image_urls.medium if hasattr(user.profile_image_urls, 'medium') else None,
            },
            "url": f"https://www.pixiv.net/users/{user_id}",
            "total_illusts": profile.total_illusts,
            "total_manga": profile.total_manga,
            "total_novels": getattr(profile, 'total_novels', 0),
            "total_bookmarks": profile.total_illust_bookmarks_public,
            "twitter_account": getattr(profile, 'twitter_account', None),
            "webpage": getattr(profile, 'webpage', None),
        }
    except Exception as e:
        return {"error": str(e)}


def search_artworks(keyword: str, limit: int = 10) -> Dict[str, Any]:
    """
    Search for artworks by keyword
    
    Args:
        keyword: Search keyword
        limit: Maximum number of results to return (default 10)
        
    Returns:
        Dictionary containing search results
    """
    if not _client._ensure_auth():
        return {"error": "Not authenticated. Set PIXIV_REFRESH_TOKEN environment variable."}
    
    try:
        result = _client.api.search_illust(keyword, search_target='partial_match_for_tags')
        
        if not hasattr(result, 'illusts'):
            return {"results": [], "total": 0}
        
        illusts = result.illusts[:limit]
        
        results = []
        for illust in illusts:
            results.append({
                "id": illust.id,
                "title": illust.title,
                "type": illust.type,
                "user_name": illust.user.name,
                "tags": [tag.name for tag in illust.tags],
                "total_bookmarks": illust.total_bookmarks,
                "url": f"https://www.pixiv.net/artworks/{illust.id}",
            })
        
        return {
            "results": results,
            "total": len(results),
            "keyword": keyword,
        }
    except Exception as e:
        return {"error": str(e)}


def get_ranking(mode: str = 'day', limit: int = 10) -> Dict[str, Any]:
    """
    Get Pixiv ranking
    
    Args:
        mode: Ranking mode ('day', 'week', 'month', 'day_male', 'day_female', 'week_rookie', etc.)
        limit: Maximum number of results to return (default 10)
        
    Returns:
        Dictionary containing ranking results
    """
    if not _client._ensure_auth():
        return {"error": "Not authenticated. Set PIXIV_REFRESH_TOKEN environment variable."}
    
    try:
        result = _client.api.illust_ranking(mode=mode)
        
        if not hasattr(result, 'illusts'):
            return {"results": [], "total": 0}
        
        illusts = result.illusts[:limit]
        
        results = []
        for illust in illusts:
            results.append({
                "id": illust.id,
                "title": illust.title,
                "type": illust.type,
                "user_name": illust.user.name,
                "tags": [tag.name for tag in illust.tags],
                "total_bookmarks": illust.total_bookmarks,
                "url": f"https://www.pixiv.net/artworks/{illust.id}",
            })
        
        return {
            "results": results,
            "total": len(results),
            "mode": mode,
        }
    except Exception as e:
        return {"error": str(e)}


def get_user_illusts(user_id: int, limit: int = 10) -> Dict[str, Any]:
    """
    Get artworks by a specific user
    
    Args:
        user_id: Pixiv user ID
        limit: Maximum number of results to return (default 10)
        
    Returns:
        Dictionary containing user's artworks
    """
    if not _client._ensure_auth():
        return {"error": "Not authenticated. Set PIXIV_REFRESH_TOKEN environment variable."}
    
    try:
        result = _client.api.user_illusts(user_id)
        
        if not hasattr(result, 'illusts'):
            return {"results": [], "total": 0}
        
        illusts = result.illusts[:limit]
        
        results = []
        for illust in illusts:
            results.append({
                "id": illust.id,
                "title": illust.title,
                "type": illust.type,
                "tags": [tag.name for tag in illust.tags],
                "create_date": illust.create_date,
                "total_bookmarks": illust.total_bookmarks,
                "url": f"https://www.pixiv.net/artworks/{illust.id}",
            })
        
        return {
            "results": results,
            "total": len(results),
            "user_id": user_id,
        }
    except Exception as e:
        return {"error": str(e)}

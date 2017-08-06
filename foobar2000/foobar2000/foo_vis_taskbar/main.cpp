#include "foo_vis_taskbar.h"

#pragma comment(lib, "..\\shared\\shared.lib")

// Foobar2K Component Variables
#define COMPONENT_NAME "foo_vis_taskbar"
#define COMPONENT_VERSION "0.1"
#define COMPONENT_DESCRIPTION "foo_vis_taskbar (c) vDk. 2010.\n\nfoo_vis_taskbar displays a visualization behind MS Windows 7 taskbar."

DECLARE_COMPONENT_VERSION(COMPONENT_NAME, COMPONENT_VERSION, COMPONENT_DESCRIPTION);

class initquit_foo_vis_taskbar : public initquit
{
	virtual void on_init() 
	{
		foo_vis_taskbar::SetAppName(APPNAME);
		foo_vis_taskbar::InitTrace();
		foo_vis_taskbar::Instance.Start();
	}

	virtual void on_quit() 
	{
		foo_vis_taskbar::Instance.Stop();
		foo_vis_taskbar::DeInitTrace();
	}
};

static initquit_factory_t< initquit_foo_vis_taskbar > foo_initquit_foo_vis_taskbar;

class preferences_page_foo_vis_taskbar_instance : public preferences_page_instance
{
private:
	HWND								m_hWnd;
	preferences_page_callback::ptr		m_Callback;

public:
	preferences_page_foo_vis_taskbar_instance(HWND parent, preferences_page_callback::ptr callback) : m_Callback(callback)
	{
		foo_vis_taskbar::Instance.CloneSettings();
		m_hWnd = foo_vis_taskbar::Instance.CreatePreferencesDialog(parent);
	}

	virtual t_uint32 get_state()
	{
		return preferences_state::resettable | preferences_state::changed;
	}
	
	virtual HWND get_wnd()
	{
		return m_hWnd;
	}
	
	virtual void apply()
	{
		foo_vis_taskbar::Instance.UpdateSettings();
		m_Callback->on_state_changed();
	}
	
	virtual void reset()
	{
		foo_vis_taskbar::Instance.CloneSettings();
		SendMessage(m_hWnd, WM_INITDIALOG, 0, 0);
		InvalidateRect(m_hWnd, NULL, TRUE);
		UpdateWindow(m_hWnd);
		m_Callback->on_state_changed();
	}
};

class preferences_page_foo_vis_taskbar : public preferences_page_v3
{
public:
	virtual const char * get_name() 
	{ 
		return "foo_vis_taskbar"; 
	}

	virtual GUID get_guid()	
	{ 
		// {C3DC676D-AC31-45A6-AA3A-7EA0DD766D57}
		static const GUID guid = { 0xc3dc676d, 0xac31, 0x45a6, { 0xaa, 0x3a, 0x7e, 0xa0, 0xdd, 0x76, 0x6d, 0x57 } };
		return guid;	
	}

	virtual GUID get_parent_guid() 
	{ 
		return preferences_page_v3::guid_display;	
	}

	virtual preferences_page_instance::ptr instantiate(HWND parent, preferences_page_callback::ptr callback)
    {
        return new service_impl_t<preferences_page_foo_vis_taskbar_instance>(parent, callback);
    }
};

static preferences_page_factory_t<preferences_page_foo_vis_taskbar> foo_preferences_page_foo_vis_taskbar;

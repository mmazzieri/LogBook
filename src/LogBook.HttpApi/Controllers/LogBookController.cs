using LogBook.Localization;
using Volo.Abp.AspNetCore.Mvc;

namespace LogBook.Controllers
{
    /* Inherit your controllers from this class.
     */
    public abstract class LogBookController : AbpController
    {
        protected LogBookController()
        {
            LocalizationResource = typeof(LogBookResource);
        }
    }
}
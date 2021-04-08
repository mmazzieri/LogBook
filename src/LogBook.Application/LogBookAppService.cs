using System;
using System.Collections.Generic;
using System.Text;
using LogBook.Localization;
using Volo.Abp.Application.Services;

namespace LogBook
{
    /* Inherit your application services from this class.
     */
    public abstract class LogBookAppService : ApplicationService
    {
        protected LogBookAppService()
        {
            LocalizationResource = typeof(LogBookResource);
        }
    }
}
